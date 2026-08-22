use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper::header::CONTENT_TYPE;
use hyper::server::conn::http1;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::Serialize;
use serde_json::{Value, json};
use tode_core::with_fallbacks;
use tode_profile::{Editor, ImportReport, ProfilePaths, install_theme, run_import};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

use crate::shortcut_manager::manager_token;

const REQUEST_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportReportRow {
    pub kind: &'static str,
    pub label: &'static str,
    pub value: String,
}

#[derive(Debug)]
struct ImportShared {
    editors: Vec<Editor>,
    paths: ProfilePaths,
    served: Arc<AtomicBool>,
    done: watch::Sender<bool>,
    token: String,
    imported: Mutex<Option<String>>,
}

#[derive(Debug)]
pub struct ImportManager {
    address: SocketAddr,
    token: String,
    served: Arc<AtomicBool>,
    done: watch::Receiver<bool>,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl ImportManager {
    pub async fn start(editors: Vec<Editor>, paths: ProfilePaths) -> std::io::Result<Self> {
        let listener =
            TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await?;
        let address = listener.local_addr()?;
        let token = manager_token()?;
        let served = Arc::new(AtomicBool::new(false));
        let (done_tx, done) = watch::channel(false);
        let shared = Arc::new(ImportShared {
            editors,
            paths,
            served: served.clone(),
            done: done_tx,
            token: token.clone(),
            imported: Mutex::new(None),
        });
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            loop {
                let accepted = tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => accepted,
                };
                let Ok((stream, _)) = accepted else {
                    break;
                };
                let shared = shared.clone();
                tokio::spawn(async move {
                    let service =
                        hyper::service::service_fn(move |request| handle(request, shared.clone()));
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });
        Ok(Self {
            address,
            token,
            served,
            done,
            shutdown: Some(shutdown_tx),
            task: Some(task),
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn url(&self) -> String {
        format!("http://{}/{}/", self.address, self.token)
    }

    pub fn served(&self) -> bool {
        self.served.load(Ordering::SeqCst)
    }

    pub async fn wait_done(&mut self) {
        while !*self.done.borrow() {
            if self.done.changed().await.is_err() {
                break;
            }
        }
    }

    pub async fn close(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for ImportManager {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

async fn handle(
    request: Request<Incoming>,
    shared: Arc<ImportShared>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = request.method().clone();
    let scoped = format!("/{}/", shared.token);
    let raw = request.uri().path();
    let path = if raw == scoped || raw == scoped.trim_end_matches('/') {
        "/".to_owned()
    } else if let Some(rest) = raw.strip_prefix(&scoped) {
        format!("/{rest}")
    } else {
        return Ok(text(
            StatusCode::NOT_FOUND,
            "text/plain; charset=utf-8",
            "not found\n",
        ));
    };
    let response = match (method, path.as_str()) {
        (Method::GET, "/") => {
            shared.served.store(true, Ordering::SeqCst);
            text(StatusCode::OK, "text/html; charset=utf-8", IMPORT_HTML)
        }
        (Method::GET, "/state") => json_response(
            StatusCode::OK,
            &json!({
                "editors": shared.editors.iter().map(|editor| json!({
                    "name": editor.name,
                    "icon": null,
                })).collect::<Vec<_>>(),
                "imported": shared.imported.lock().expect("import state lock").clone(),
            }),
        ),
        (Method::POST, "/import") => match read_json(request).await {
            Ok(sent) => {
                let name = sent["name"].as_str().unwrap_or_default();
                let Some(editor) = shared.editors.iter().find(|editor| editor.name == name) else {
                    return Ok(json_response(
                        StatusCode::OK,
                        &json!({"ok": false, "warning": format!("{name} is not one of the choices")}),
                    ));
                };
                let report = run_import(editor, &shared.paths);
                if let Err(error) = install_theme(&shared.paths, &with_fallbacks(None)) {
                    return Ok(json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &json!({"ok": false, "warning": error.to_string()}),
                    ));
                }
                *shared.imported.lock().expect("import state lock") = Some(name.into());
                json_response(
                    StatusCode::OK,
                    &json!({"ok": true, "rows": report_rows(&report)}),
                )
            }
            Err(response) => *response,
        },
        (Method::POST, "/done") => {
            let _ = shared.done.send(true);
            json_response(StatusCode::OK, &json!({"ok": true, "next": null}))
        }
        _ => text(
            StatusCode::NOT_FOUND,
            "text/plain; charset=utf-8",
            "not found\n",
        ),
    };
    Ok(response)
}

pub fn report_rows(report: &ImportReport) -> Vec<ImportReportRow> {
    let mut rows = Vec::new();
    if !report.extensions.copied.is_empty() {
        rows.push(ImportReportRow {
            kind: "ok",
            label: "extensions",
            value: format!("{} copied", report.extensions.copied.len()),
        });
    }
    for skipped in &report.extensions.skipped {
        rows.push(ImportReportRow {
            kind: "warn",
            label: "skipped",
            value: format!("{} — {}", skipped.id, skipped.why),
        });
    }
    if let Some(settings) = &report.settings {
        rows.push(ImportReportRow {
            kind: "ok",
            label: "settings",
            value: format!("{} entries", settings.imported),
        });
    }
    if let Some(keybindings) = report.keybindings {
        rows.push(ImportReportRow {
            kind: "ok",
            label: "keybindings",
            value: format!("{keybindings} entries"),
        });
    }
    if !report.snippets.is_empty() {
        rows.push(ImportReportRow {
            kind: "ok",
            label: "snippets",
            value: report.snippets.join(", "),
        });
    }
    if report.tasks {
        rows.push(ImportReportRow {
            kind: "ok",
            label: "tasks",
            value: "tasks.json".into(),
        });
    }
    rows
}

async fn read_json(request: Request<Incoming>) -> Result<Value, Box<Response<Full<Bytes>>>> {
    let collected = Limited::new(request.into_body(), REQUEST_LIMIT)
        .collect()
        .await
        .map_err(|error| {
            Box::new(json_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                &json!({"ok": false, "warning": error.to_string()}),
            ))
        })?;
    serde_json::from_slice(&collected.to_bytes()).map_err(|error| {
        Box::new(json_response(
            StatusCode::BAD_REQUEST,
            &json!({"ok": false, "warning": error.to_string()}),
        ))
    })
}

fn json_response(status: StatusCode, value: &impl Serialize) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(value).expect("import response serializes"),
        )))
        .expect("valid import response")
}

fn text(status: StatusCode, content_type: &str, body: &'static str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type)
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("valid import response")
}

const IMPORT_HTML: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>tode import</title><style>
:root{color-scheme:dark;--ink:#0d0f13;--paper:#e6e9ef;--muted:#929cab;--line:#303846;--cyan:#4cb4e7;--pane:#171b22;--ok:#70c79b;--warn:#e1a44b}*{box-sizing:border-box}body{margin:0;background:var(--ink);color:var(--paper);font:15px/1.5 system-ui,-apple-system,sans-serif}.shell{width:min(760px,calc(100% - 32px));margin:auto;padding:42px 0}.eyebrow,.name,.row{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}.eyebrow{color:var(--cyan);font-size:12px;letter-spacing:.13em;text-transform:uppercase}h1{font:650 clamp(34px,7vw,62px)/.96 ui-monospace,SFMono-Regular,Menlo,monospace;letter-spacing:-.06em;margin:8px 0 14px}.intro{color:var(--muted);max-width:620px}.editors{display:grid;gap:10px;margin:30px 0}.editor{display:grid;grid-template-columns:1fr auto;align-items:center;gap:18px;padding:18px 20px;background:var(--pane);border:1px solid var(--line);border-radius:8px}.name{font-size:18px}.pick,.done,.cancel{border:1px solid var(--cyan);background:transparent;color:var(--cyan);border-radius:6px;padding:8px 13px;cursor:pointer}.pick:hover,.pick:focus-visible,.done:hover,.done:focus-visible,.cancel:hover,.cancel:focus-visible{background:var(--cyan);color:var(--ink);outline:none}.pick:disabled,.cancel:disabled{opacity:.45}.cancel{border-color:var(--line);color:var(--muted)}.report{border-top:1px solid var(--line);padding-top:18px}.row{display:grid;grid-template-columns:88px 130px 1fr;gap:12px;padding:7px 0}.row .ok{color:var(--ok)}.row .warn{color:var(--warn)}.warning{color:var(--warn);min-height:24px}.done{display:none;margin-top:22px}.done.show{display:inline-block}@media(max-width:560px){.editor{grid-template-columns:1fr}.row{grid-template-columns:70px 1fr}.row span:last-child{grid-column:2}}
</style></head><body><main class="shell"><div class="eyebrow">profile transfer</div><h1>Bring your editor with you.</h1><p class="intro">Choose one VS Code-family profile. tode preserves its managed settings and imports everything else once.</p><section id="editors" class="editors" aria-live="polite"></section><div id="warning" class="warning" role="status" aria-live="polite"></div><section id="report" class="report"></section><button id="cancel" class="cancel">Skip import</button> <button id="done" class="done">Continue</button></main><script>
const editors=document.querySelector('#editors'),report=document.querySelector('#report'),warning=document.querySelector('#warning'),done=document.querySelector('#done'),cancel=document.querySelector('#cancel');const post=async(path,payload={})=>(await fetch(path,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload)})).json();
function esc(v){return String(v??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]))}function render(state){editors.innerHTML=state.editors.map(e=>`<article class="editor"><div class="name">${esc(e.name)}</div><button class="pick" data-name="${esc(e.name)}">Import this profile</button></article>`).join('');editors.querySelectorAll('.pick').forEach(button=>button.onclick=()=>run(button.dataset.name))}
async function run(name){warning.textContent=`Importing from ${name}…`;editors.querySelectorAll('button').forEach(b=>b.disabled=true);cancel.disabled=true;const result=await post('import',{name});if(!result.ok){warning.textContent=result.warning;editors.querySelectorAll('button').forEach(b=>b.disabled=false);cancel.disabled=false;return}warning.textContent=`Imported from ${name}`;report.innerHTML=result.rows.map(r=>`<div class="row"><span class="${r.kind}">${r.kind==='ok'?'copied':'check'}</span><span>${esc(r.label)}</span><span>${esc(r.value)}</span></div>`).join('');cancel.style.display='none';done.classList.add('show')}
async function finish(label){done.disabled=true;cancel.disabled=true;await post('done');warning.textContent=label}done.onclick=()=>finish('Import complete');cancel.onclick=()=>finish('Import skipped');fetch('state').then(r=>r.json()).then(render).catch(error=>warning.textContent=String(error));
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn token_scoped_page_validates_imports_and_completes() {
        let root = TempDir::new().unwrap();
        let source = root.path().join("source/User");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("settings.json"), "{\"editor.fontSize\":19}").unwrap();
        let paths = ProfilePaths::from_environment(root.path(), &BTreeMap::new());
        let editor = Editor {
            name: "Code".into(),
            user_dir: source,
            extensions_dir: None,
        };
        let mut manager = ImportManager::start(vec![editor], paths.clone())
            .await
            .unwrap();
        let client = reqwest::Client::new();
        let origin = manager.url();
        assert_eq!(
            client
                .get(format!("http://{}/", manager.address()))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert!(
            client
                .get(&origin)
                .send()
                .await
                .unwrap()
                .status()
                .is_success()
        );
        assert!(manager.served());
        let state: Value = client
            .get(format!("{origin}state"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(state["editors"][0]["name"], "Code");
        let invalid: Value = client
            .post(format!("{origin}import"))
            .json(&json!({"name": "Missing"}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(invalid["ok"], false);
        let imported: Value = client
            .post(format!("{origin}import"))
            .json(&json!({"name": "Code"}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(imported["ok"], true);
        assert_eq!(imported["rows"][0]["label"], "settings");
        assert!(paths.user.join("settings.json").is_file());
        client
            .post(format!("{origin}done"))
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        manager.wait_done().await;
        manager.close().await;
    }
}
