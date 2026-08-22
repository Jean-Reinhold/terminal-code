use std::convert::Infallible;
use std::fs::File;
use std::io::Read;
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
use tode_core::Decision;
use tode_profile::shortcut_manager::{ClaimInfo, DecisionKind, ManagerRowKind, ShortcutSession};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

const REQUEST_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct ShortcutManagerConfig {
    pub reload_hint: String,
    pub intro: bool,
    pub continues: bool,
}

#[derive(Debug)]
pub struct ShortcutManager {
    address: SocketAddr,
    token: String,
    served: Arc<AtomicBool>,
    confirmed: Arc<AtomicBool>,
    done: watch::Receiver<bool>,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl ShortcutManager {
    pub async fn start(
        session: ShortcutSession,
        config: ShortcutManagerConfig,
    ) -> std::io::Result<Self> {
        let listener =
            TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await?;
        let address = listener.local_addr()?;
        let token = manager_token()?;
        let session = Arc::new(Mutex::new(session));
        let config = Arc::new(config);
        let served = Arc::new(AtomicBool::new(false));
        let confirmed = Arc::new(AtomicBool::new(false));
        let (done_tx, done) = watch::channel(false);
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let task_served = served.clone();
        let task_confirmed = confirmed.clone();
        let task_token = Arc::new(token.clone());
        let task = tokio::spawn(async move {
            loop {
                let accepted = tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => accepted,
                };
                let Ok((stream, _)) = accepted else {
                    break;
                };
                let session = session.clone();
                let config = config.clone();
                let served = task_served.clone();
                let confirmed = task_confirmed.clone();
                let done = done_tx.clone();
                let token = task_token.clone();
                tokio::spawn(async move {
                    let service = hyper::service::service_fn(move |request| {
                        handle(
                            request,
                            session.clone(),
                            config.clone(),
                            served.clone(),
                            confirmed.clone(),
                            done.clone(),
                            token.clone(),
                        )
                    });
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
            confirmed,
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

    pub fn confirmed(&self) -> bool {
        self.confirmed.load(Ordering::SeqCst)
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

impl Drop for ShortcutManager {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

async fn handle(
    request: Request<Incoming>,
    session: Arc<Mutex<ShortcutSession>>,
    config: Arc<ShortcutManagerConfig>,
    served: Arc<AtomicBool>,
    confirmed: Arc<AtomicBool>,
    done: watch::Sender<bool>,
    token: Arc<String>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = request.method().clone();
    let scoped = format!("/{token}");
    let scoped_prefix = format!("{scoped}/");
    let raw_path = request.uri().path();
    let path = if raw_path == scoped || raw_path == scoped_prefix {
        "/".to_owned()
    } else if let Some(rest) = raw_path.strip_prefix(&scoped_prefix) {
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
            served.store(true, Ordering::SeqCst);
            text(StatusCode::OK, "text/html; charset=utf-8", MANAGER_HTML)
        }
        (Method::GET, "/state") => {
            let session = session.lock().expect("shortcut session lock");
            json_response(
                StatusCode::OK,
                &json!({
                    "rows": session.rows(),
                    "terminalName": session.provider().name,
                    "reloadHint": config.reload_hint,
                    "intro": config.intro,
                    "continues": config.continues,
                    "logos": {"terminal": null, "editor": null},
                }),
            )
        }
        (Method::POST, "/taken") => match read_json(request).await {
            Ok(sent) => {
                let raw = sent["chord"].as_str().unwrap_or_default();
                let session = session.lock().expect("shortcut session lock");
                let Some(chord) = session.normalize(raw) else {
                    return Ok(json_response(
                        StatusCode::OK,
                        &json!({"ok": false, "warning": format!("{raw} does not parse as a chord")}),
                    ));
                };
                let id = sent["id"].as_str();
                let command = sent["command"].as_str();
                let side = match sent["side"].as_str() {
                    Some("terminal") => Some(ManagerRowKind::Terminal),
                    Some("editor") => Some(ManagerRowKind::Import),
                    _ => None,
                };
                if chord != id.unwrap_or_default()
                    && let Some(info) = session.taken(&chord, id, command, side)
                {
                    json_response(
                        StatusCode::OK,
                        &json!({
                            "ok": false,
                            "warning": format!("{chord} is already bound to {}", info.holder),
                            "claim": info.claim,
                            "chord": chord,
                        }),
                    )
                } else {
                    json_response(StatusCode::OK, &json!({"ok": true, "chord": chord}))
                }
            }
            Err(response) => *response,
        },
        (Method::POST, "/decide") => match read_json(request).await {
            Ok(sent) => {
                let id = sent["id"].as_str().unwrap_or_default();
                let kind = match sent["kind"].as_str() {
                    Some("claim") => DecisionKind::Claim,
                    Some("import") => DecisionKind::Import,
                    _ => DecisionKind::Terminal,
                };
                let mut decision = if sent["decision"].is_null() {
                    None
                } else {
                    match serde_json::from_value::<Decision>(sent["decision"].clone()) {
                        Ok(decision) => Some(decision),
                        Err(error) => {
                            return Ok(json_response(
                                StatusCode::BAD_REQUEST,
                                &json!({"ok": false, "warning": error.to_string()}),
                            ));
                        }
                    }
                };
                let mut session = session.lock().expect("shortcut session lock");
                if let Some(decision) = &mut decision
                    && decision.choice == tode_core::DecisionChoice::Editor
                    && let Some(key) = decision.key.as_deref()
                {
                    let Some(normalized) = session.normalize(key) else {
                        return Ok(json_response(
                            StatusCode::OK,
                            &json!({
                                "ok": false,
                                "warning": format!("{key} does not parse"),
                                "rows": session.rows(),
                            }),
                        ));
                    };
                    decision.key = Some(normalized);
                }
                let info = sent["action"].as_str().map(|command| ClaimInfo {
                    command: command.into(),
                    when: sent["guard"].as_str().map(str::to_owned),
                });
                session.decide(
                    id,
                    kind,
                    decision,
                    sent["side"].as_str() == Some("claim"),
                    info,
                );
                json_response(StatusCode::OK, &json!({"ok": true, "rows": session.rows()}))
            }
            Err(response) => *response,
        },
        (Method::POST, "/confirm") => {
            let result = session.lock().expect("shortcut session lock").confirm();
            match result {
                Ok(_) => {
                    confirmed.store(true, Ordering::SeqCst);
                    json_response(
                        StatusCode::OK,
                        &json!({"ok": true, "note": format!("applied — {}", config.reload_hint)}),
                    )
                }
                Err(error) => json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &json!({"ok": false, "warning": error.to_string()}),
                ),
            }
        }
        (Method::POST, "/done") => {
            let _ = done.send(true);
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

fn manager_token() -> std::io::Result<String> {
    let mut bytes = [0_u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(hex::encode(bytes))
}

async fn read_json(request: Request<Incoming>) -> Result<Value, Box<Response<Full<Bytes>>>> {
    let limited = Limited::new(request.into_body(), REQUEST_LIMIT);
    let collected = limited.collect().await.map_err(|error| {
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
    let bytes = serde_json::to_vec(value).expect("manager JSON serializes");
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(bytes)))
        .expect("valid manager response")
}

fn text(status: StatusCode, content_type: &str, body: &'static str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type)
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("valid manager response")
}

const MANAGER_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>tode shortcut switchyard</title>
<style>
:root{color-scheme:dark;--ink:#0d0f13;--pane:#171b22;--lift:#202631;--paper:#e6e9ef;--muted:#929cab;--cyan:#4cb4e7;--amber:#e1a44b;--fault:#e46b6b;--line:#303846}
*{box-sizing:border-box}body{margin:0;background:var(--ink);color:var(--paper);font:15px/1.5 system-ui,-apple-system,sans-serif}button,input{font:inherit}button{color:inherit}
.shell{width:min(920px,calc(100% - 32px));margin:auto;padding:38px 0 28px}.mast{display:grid;grid-template-columns:1fr auto;gap:24px;align-items:end;border-bottom:1px solid var(--line);padding-bottom:22px;margin-bottom:26px}
.eyebrow,.chord,.action,.status{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}.eyebrow{color:var(--cyan);letter-spacing:.12em;text-transform:uppercase;font-size:12px}.mast h1{font:650 clamp(30px,6vw,58px)/.95 ui-monospace,SFMono-Regular,Menlo,monospace;letter-spacing:-.06em;margin:8px 0 0;max-width:680px}.count{color:var(--muted);text-align:right}.count b{display:block;color:var(--paper);font:600 32px/1 ui-monospace,monospace}
.intro{color:var(--muted);max-width:680px;margin:0 0 24px}.stack{display:grid;gap:14px}.row{background:var(--pane);border:1px solid var(--line);border-radius:10px;overflow:hidden}.row[data-decided=true]{border-color:#42667a}.route{display:grid;grid-template-columns:1fr 110px 1fr;align-items:stretch;min-height:150px}.side{padding:20px}.side.left{text-align:right}.side h2{font-size:14px;margin:0 0 8px;color:var(--muted);font-weight:500}.action{font-size:16px;color:var(--paper);overflow-wrap:anywhere}.detail{font-size:13px;color:var(--muted);margin-top:8px}.hinge{position:relative;display:grid;place-items:center;border-inline:1px solid var(--line);background:#11151b}.hinge:before,.hinge:after{content:"";position:absolute;top:50%;width:24px;height:1px;background:var(--line)}.hinge:before{right:100%}.hinge:after{left:100%}.chord{position:relative;z-index:1;padding:8px 10px;background:var(--ink);border:1px solid var(--cyan);color:var(--cyan);border-radius:6px;white-space:nowrap}
.controls{display:flex;flex-wrap:wrap;gap:8px;margin-top:14px}.left .controls{justify-content:flex-end}.choice,.move{border:1px solid var(--line);background:var(--lift);border-radius:6px;padding:7px 10px;cursor:pointer}.choice:hover,.choice:focus-visible,.move:hover,.move:focus-visible{border-color:var(--cyan);outline:none}.choice.active{background:#153445;border-color:var(--cyan)}.movebox{display:flex;gap:6px;margin-top:8px}.movebox input{min-width:0;width:150px;background:var(--ink);border:1px solid var(--line);border-radius:6px;color:var(--paper);padding:7px 9px;font-family:ui-monospace,monospace}.movebox input:focus{outline:1px solid var(--amber);border-color:var(--amber)}
.claims{border-top:1px solid var(--line);padding:12px 20px;color:var(--muted);font-size:13px}.claims:empty{display:none}.claim{display:flex;gap:9px;align-items:baseline}.claim .action{font-size:13px}.warning{color:var(--fault);min-height:24px;margin-top:12px}.dock{position:static;background:color-mix(in srgb,var(--ink) 92%,transparent);backdrop-filter:blur(12px);border-top:1px solid var(--line)}.dockin{width:min(920px,calc(100% - 32px));margin:auto;padding:14px 0;display:flex;align-items:center;gap:12px}.status{color:var(--muted);margin-right:auto}.primary{border:1px solid var(--cyan);background:var(--cyan);color:var(--ink);font-weight:700;border-radius:6px;padding:9px 15px;cursor:pointer}.primary:disabled{opacity:.4;cursor:not-allowed}.finish{display:none}.finish.show{display:block}
.route.moving .hinge:after{background:var(--amber);box-shadow:0 0 10px var(--amber);animation:pulse .7s ease-out}@keyframes pulse{from{transform:scaleX(.2);transform-origin:left}to{transform:scaleX(1)}}@media(prefers-reduced-motion:reduce){*{animation:none!important;scroll-behavior:auto!important}}@media(max-width:680px){.mast{grid-template-columns:1fr}.count{text-align:left}.route{grid-template-columns:1fr}.hinge{min-height:66px;border-inline:0;border-block:1px solid var(--line)}.side.left{text-align:left}.left .controls{justify-content:flex-start}.hinge:before,.hinge:after{display:none}}
</style>
</head>
<body><main class="shell"><header class="mast"><div><div class="eyebrow">shortcut switchyard</div><h1>Route every chord once.</h1></div><div class="count"><b id="count">—</b>conflicts</div></header><p class="intro">Each row shows what the terminal catches and what the editor needs. Give the chord to the editor, move the editor command, or deliberately leave the collision.</p><section id="rows" class="stack" aria-live="polite"></section><div id="warning" class="warning" role="alert"></div></main><footer class="dock"><div class="dockin"><div id="status" class="status">Loading keymap…</div><button id="confirm" class="primary" disabled>Apply routes</button><button id="finish" class="primary finish">Close manager</button></div></footer>
<script>
const rowsEl=document.querySelector('#rows'),warning=document.querySelector('#warning'),statusEl=document.querySelector('#status'),confirmBtn=document.querySelector('#confirm'),finishBtn=document.querySelector('#finish');let state;
const post=async(path,payload={})=>{const r=await fetch(path,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload)});return r.json()};
const decision=(choice,key=null)=>({choice,key,action:null,guard:null,owner_terminal:false,command:null});
function decided(row){return !!row.decision||!!row.claimDecision}
function esc(value){return String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]))}
function claims(row){return (row.claims||[]).map(c=>`<div class="claim"><span class="chord">${esc(c.chord)}</span><span class="action">${esc(c.describes||c.command)}</span><span>${esc(c.claimant)}</span></div>`).join('')}
function render(){rowsEl.innerHTML=state.rows.map((row,i)=>{const terminal=row.kind==='terminal';const left=row.terminal.short||row.importedCommand||'imported binding';const right=row.means;return `<article class="row" data-id="${esc(row.id)}" data-kind="${row.kind}" data-decided="${decided(row)}"><div class="route"><section class="side left"><h2>${esc(row.terminal.name)} behavior</h2><div class="action">${esc(left)}</div><div class="detail">${esc(row.terminal.does||row.claimant||'Imported editor binding')}</div><div class="controls"><button class="choice give">Give chord to editor</button><button class="choice keep">Keep collision</button></div></section><div class="hinge"><span class="chord">${esc(row.id)}</span></div><section class="side"><h2>terminal-code behavior</h2><div class="action">${esc(right)}</div><div class="detail">${esc(row.detail?.when||row.claimDescribes||row.detail?.command||'Editor command')}</div><div class="movebox"><input aria-label="Replacement chord" placeholder="new chord"><button class="move">Move editor</button></div></section></div><div class="claims">${claims(row)}</div></article>`}).join('');document.querySelector('#count').textContent=state.rows.length;statusEl.textContent=`${state.terminalName} · ${state.rows.filter(decided).length}/${state.rows.length} routed`;confirmBtn.disabled=state.rows.some(r=>!decided(r));bind()}
async function send(row,payload){warning.textContent='';const result=await post('decide',{id:row.dataset.id,kind:row.dataset.kind,side:'own',...payload});if(!result.ok){warning.textContent=result.warning;return}state.rows=result.rows;render()}
function bind(){document.querySelectorAll('.row').forEach(row=>{const item=state.rows.find(r=>r.id===row.dataset.id);row.querySelector('.give').onclick=()=>send(row,{decision:decision('terminal')});row.querySelector('.keep').onclick=()=>send(row,{decision:decision('keep')});row.querySelector('.move').onclick=async()=>{const input=row.querySelector('input'),checked=await post('taken',{chord:input.value,id:item.id,side:'editor'});if(!checked.ok){warning.textContent=checked.warning;input.focus();return}row.classList.add('moving');await send(row,{decision:decision('editor',checked.chord)})}})}
confirmBtn.onclick=async()=>{const result=await post('confirm');if(!result.ok){warning.textContent=result.warning;return}statusEl.textContent=result.note;confirmBtn.disabled=true;finishBtn.classList.add('show')};finishBtn.onclick=async()=>{await post('done');statusEl.textContent='Applied. You can close this pane.';finishBtn.disabled=true};
fetch('state').then(r=>r.json()).then(value=>{state=value;render()}).catch(error=>{warning.textContent=String(error);statusEl.textContent='Manager unavailable'});
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsString as OS;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use tempfile::TempDir;
    use tode_core::{DecisionChoice, parse_jsonc};
    use tode_profile::ProfilePaths;
    use tode_profile::shortcuts::{detect_provider, load_decisions};

    use super::*;

    #[tokio::test]
    async fn serves_and_applies_full_manager_protocol() {
        let root = TempDir::new().unwrap();
        let bin = root.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let ghostty = bin.join("ghostty");
        fs::write(
            &ghostty,
            "#!/bin/sh\nprintf 'keybind = ctrl+p=new_tab\\n'\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&ghostty).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&ghostty, permissions).unwrap();
        let environment = BTreeMap::from([
            (OS::from("PATH"), bin.into_os_string()),
            (OS::from("TERM_PROGRAM"), OS::from("ghostty")),
            (
                OS::from("XDG_DATA_HOME"),
                root.path().join("data").into_os_string(),
            ),
            (
                OS::from("XDG_CONFIG_HOME"),
                root.path().join("config").into_os_string(),
            ),
        ]);
        let paths = ProfilePaths::from_environment(root.path(), &environment);
        fs::create_dir_all(&paths.user).unwrap();
        fs::write(
            paths.user.join("keybindings.json"),
            r#"[{"key":"ctrl+p","command":"workbench.action.quickOpen"}]"#,
        )
        .unwrap();
        let provider = detect_provider(root.path(), &environment).unwrap();
        let session = ShortcutSession::new(provider, paths.clone()).unwrap();
        let mut manager = ShortcutManager::start(
            session,
            ShortcutManagerConfig {
                reload_hint: "reload Ghostty".into(),
                intro: false,
                continues: false,
            },
        )
        .await
        .unwrap();
        let client = reqwest::Client::new();
        let origin = manager.url();
        let bare = client
            .get(format!("http://{}/", manager.address()))
            .send()
            .await
            .unwrap();
        assert_eq!(bare.status(), StatusCode::NOT_FOUND);
        let page = client
            .get(&origin)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(page.contains("shortcut switchyard"));
        assert!(manager.served());
        let state: Value = client
            .get(format!("{origin}state"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(state["rows"].as_array().unwrap().len(), 1);
        let decided: Value = client
            .post(format!("{origin}decide"))
            .json(&json!({
                "id": "ctrl+p",
                "kind": "terminal",
                "decision": {
                    "choice": "terminal",
                    "key": null,
                    "action": null,
                    "guard": null,
                    "owner_terminal": false,
                    "command": null
                }
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(decided["rows"][0]["decision"]["choice"], "terminal");
        let confirmed: Value = client
            .post(format!("{origin}confirm"))
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(confirmed["ok"], true);
        assert!(manager.confirmed());
        let saved = load_decisions(&paths).unwrap();
        assert_eq!(saved.choices["ctrl+p"].choice, DecisionChoice::Terminal);
        let keybindings: Vec<tode_core::Binding> =
            parse_jsonc(&fs::read_to_string(paths.user.join("keybindings.json")).unwrap()).unwrap();
        assert!(
            keybindings
                .iter()
                .any(|binding| binding.command == "workbench.action.quickOpen")
        );
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
