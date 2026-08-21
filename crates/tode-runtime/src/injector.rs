use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::client::conn::http1 as client_http1;
use hyper::header::{
    ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, HOST,
    HeaderValue, ORIGIN, REFERER, TRANSFER_ENCODING,
};
use hyper::server::conn::http1 as server_http1;
use hyper::service::service_fn;
use hyper::{HeaderMap, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

pub const FONT_ROUTE: &str = "/__tode/font.ttf";
pub const FONT_FALLBACKS: &str = "Menlo, \"DejaVu Sans Mono\", \"Liberation Mono\", monospace";

type ProxyBody = UnsyncBoxBody<Bytes, hyper::Error>;

#[derive(Debug, Clone)]
pub struct InjectorConfig {
    pub listen: SocketAddr,
    pub upstream: SocketAddr,
    pub css_file: PathBuf,
    pub font_file: Option<PathBuf>,
    pub hold: Duration,
}

impl InjectorConfig {
    pub fn local(upstream_port: u16, css_file: PathBuf) -> Self {
        Self {
            listen: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            upstream: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), upstream_port),
            css_file,
            font_file: None,
            hold: Duration::from_secs(20),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InjectorError {
    #[error("bind injector: {0}")]
    Bind(std::io::Error),
    #[error("read injector address: {0}")]
    Address(std::io::Error),
}

#[derive(Debug)]
pub struct Injector {
    address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

#[derive(Debug)]
struct State {
    config: InjectorConfig,
    started: Instant,
    ever_answered: AtomicBool,
}

impl Injector {
    pub async fn start(config: InjectorConfig) -> Result<Self, InjectorError> {
        let listener = TcpListener::bind(config.listen)
            .await
            .map_err(InjectorError::Bind)?;
        let address = listener.local_addr().map_err(InjectorError::Address)?;
        let state = Arc::new(State {
            config,
            started: Instant::now(),
            ever_answered: AtomicBool::new(false),
        });
        let (shutdown, mut stop) = oneshot::channel();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut stop => break,
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { break };
                        let state = state.clone();
                        tokio::spawn(async move {
                            let service = service_fn(move |request| handle(request, state.clone()));
                            let connection = server_http1::Builder::new()
                                .serve_connection(TokioIo::new(stream), service)
                                .with_upgrades();
                            let _ = connection.await;
                        });
                    }
                }
            }
        });
        Ok(Self {
            address,
            shutdown: Some(shutdown),
            task,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub async fn shutdown(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let _ = self.task.await;
    }
}

pub fn injected_css(background: &str, font_family: &str) -> String {
    let stack = format!("\"{font_family}\", {FONT_FALLBACKS}");
    [
        format!(
            "@font-face{{font-family:\"{font_family}\";src:url(\"{FONT_ROUTE}\") format(\"truetype\");font-weight:100 900;font-display:block;}}"
        ),
        format!("html,body{{background:{background} !important;}}"),
        "html{overflow:hidden;}".into(),
        "body{margin:0;}".into(),
        format!(
            ".monaco-workbench{{background:{background};font-family:{stack} !important;}}"
        ),
        ".monaco-workbench .part,.monaco-workbench .monaco-list,.monaco-workbench .monaco-inputbox,".into(),
        ".monaco-workbench input,.monaco-workbench select,.monaco-workbench textarea,".into(),
        ".monaco-menu,.quick-input-widget,.monaco-hover,.notifications-toasts".into(),
        format!("{{font-family:{stack} !important;}}"),
        format!(":root{{--monaco-monospace-font:{stack};}}"),
        ".editor-group-watermark{display:none !important;}".into(),
    ]
    .join("")
}

async fn handle(
    mut request: Request<Incoming>,
    state: Arc<State>,
) -> Result<Response<ProxyBody>, Infallible> {
    if request.uri().path().starts_with(FONT_ROUTE) {
        return Ok(font_response(&state).await);
    }

    let client_upgrade = is_upgrade(&request).then(|| hyper::upgrade::on(&mut request));
    let wants_html = request
        .headers()
        .get(hyper::header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("text/html"));
    let method = request.method().clone();
    let path = request
        .uri()
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/")
        .to_owned();
    let headers = forward_headers(request.headers(), state.config.upstream, wants_html);
    let body = match request.into_body().collect().await {
        Ok(body) => body.to_bytes(),
        Err(_) => {
            return Ok(text_response(
                StatusCode::BAD_REQUEST,
                "tode: request body failed\n",
            ));
        }
    };

    loop {
        let stream = match TcpStream::connect(state.config.upstream).await {
            Ok(stream) => stream,
            Err(error)
                if error.kind() == std::io::ErrorKind::ConnectionRefused
                    && !state.ever_answered.load(Ordering::SeqCst)
                    && state.started.elapsed() < state.config.hold =>
            {
                tokio::time::sleep(Duration::from_millis(60)).await;
                continue;
            }
            Err(_) => {
                return Ok(text_response(
                    StatusCode::BAD_GATEWAY,
                    "tode: code-server is not answering\n",
                ));
            }
        };
        let (mut sender, connection) = match client_http1::handshake(TokioIo::new(stream)).await {
            Ok(parts) => parts,
            Err(_) => {
                return Ok(text_response(
                    StatusCode::BAD_GATEWAY,
                    "tode: code-server is not answering\n",
                ));
            }
        };
        tokio::spawn(async move {
            let _ = connection.with_upgrades().await;
        });
        let mut upstream_request = Request::builder()
            .method(method.clone())
            .uri(path.clone())
            .body(Full::new(body.clone()))
            .expect("validated request parts");
        *upstream_request.headers_mut() = headers.clone();
        let mut upstream = match sender.send_request(upstream_request).await {
            Ok(response) => response,
            Err(_) => {
                return Ok(text_response(
                    StatusCode::BAD_GATEWAY,
                    "tode: code-server is not answering\n",
                ));
            }
        };
        state.ever_answered.store(true, Ordering::SeqCst);

        if upstream.status() == StatusCode::SWITCHING_PROTOCOLS {
            if let Some(client_upgrade) = client_upgrade {
                let upstream_upgrade = hyper::upgrade::on(&mut upstream);
                tokio::spawn(async move {
                    let (Ok(client), Ok(upstream)) = tokio::join!(client_upgrade, upstream_upgrade)
                    else {
                        return;
                    };
                    let mut client = TokioIo::new(client);
                    let mut upstream = TokioIo::new(upstream);
                    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
                });
            }
            return Ok(response_with_empty_body(upstream));
        }

        let content_type = upstream
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let css = tokio::fs::read_to_string(&state.config.css_file)
            .await
            .unwrap_or_default();
        if !content_type.contains("text/html") || css.is_empty() {
            let (parts, body) = upstream.into_parts();
            return Ok(Response::from_parts(parts, body.boxed_unsync()));
        }
        let status = upstream.status();
        let mut response_headers = upstream.headers().clone();
        let upstream_body = match upstream.into_body().collect().await {
            Ok(body) => body.to_bytes(),
            Err(_) => {
                return Ok(text_response(
                    StatusCode::BAD_GATEWAY,
                    "tode: code-server response failed\n",
                ));
            }
        };
        let patched = patch_html(&upstream_body, &css);
        response_headers.remove(CONTENT_ENCODING);
        response_headers.remove(TRANSFER_ENCODING);
        response_headers.insert(
            CONTENT_LENGTH,
            HeaderValue::from_str(&patched.len().to_string()).expect("length is a valid header"),
        );
        let mut response = Response::new(full_body(patched));
        *response.status_mut() = status;
        *response.headers_mut() = response_headers;
        return Ok(response);
    }
}

async fn font_response(state: &State) -> Response<ProxyBody> {
    let Some(path) = &state.config.font_file else {
        return empty_response(StatusCode::NOT_FOUND);
    };
    match tokio::fs::read(path).await {
        Ok(font) => {
            let length = font.len();
            let mut response = Response::new(full_body(font));
            *response.status_mut() = StatusCode::OK;
            response
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("font/ttf"));
            response.headers_mut().insert(
                CONTENT_LENGTH,
                HeaderValue::from_str(&length.to_string()).expect("font length is a valid header"),
            );
            response.headers_mut().insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            response
        }
        Err(_) => empty_response(StatusCode::NOT_FOUND),
    }
}

fn forward_headers(incoming: &HeaderMap, upstream: SocketAddr, wants_html: bool) -> HeaderMap {
    let mut headers = incoming.clone();
    let host = upstream.to_string();
    headers.insert(
        HOST,
        HeaderValue::from_str(&host).expect("socket address is a valid host"),
    );
    for name in [ORIGIN, REFERER] {
        if let Some(value) = headers.get(&name).and_then(|value| value.to_str().ok())
            && let Some(rewritten) = rewrite_origin(value, &host)
            && let Ok(value) = HeaderValue::from_str(&rewritten)
        {
            headers.insert(name, value);
        }
    }
    if wants_html {
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    }
    headers
}

fn rewrite_origin(value: &str, host: &str) -> Option<String> {
    let scheme = value.find("://")?;
    let after_host = value[scheme + 3..]
        .find('/')
        .map(|offset| &value[scheme + 3 + offset..])
        .unwrap_or("");
    Some(format!("http://{host}{after_host}"))
}

fn patch_html(body: &[u8], css: &str) -> Vec<u8> {
    let body = String::from_utf8_lossy(body);
    let style = format!("<style id=\"tode-injected\">{css}</style>");
    if body.contains("</head>") {
        body.replacen("</head>", &format!("{style}</head>"), 1)
            .into_bytes()
    } else {
        format!("{style}{body}").into_bytes()
    }
}

fn is_upgrade(request: &Request<Incoming>) -> bool {
    request.headers().get(hyper::header::UPGRADE).is_some()
}

fn response_with_empty_body(response: Response<Incoming>) -> Response<ProxyBody> {
    let (parts, _) = response.into_parts();
    Response::from_parts(parts, full_body(Bytes::new()))
}

fn empty_response(status: StatusCode) -> Response<ProxyBody> {
    let mut response = Response::new(full_body(Bytes::new()));
    *response.status_mut() = status;
    response
}

fn text_response(status: StatusCode, text: &'static str) -> Response<ProxyBody> {
    let mut response = Response::new(full_body(text));
    *response.status_mut() = status;
    response
}

fn full_body(value: impl Into<Bytes>) -> ProxyBody {
    Full::new(value.into())
        .map_err(|never| match never {})
        .boxed_unsync()
}
#[cfg(test)]
mod tests {
    use http_body_util::{BodyExt, Full};
    use hyper::header::{ACCEPT, ACCEPT_ENCODING};
    use hyper::service::service_fn;
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::mpsc;

    use super::*;

    #[derive(Clone)]
    struct Reply {
        status: StatusCode,
        content_type: &'static str,
        body: Bytes,
        content_encoding: Option<&'static str>,
    }

    async fn start_upstream(
        reply: Reply,
    ) -> (
        SocketAddr,
        mpsc::UnboundedReceiver<HeaderMap>,
        JoinHandle<()>,
    ) {
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let (seen, requests) = mpsc::unbounded_channel();
        let task = serve_upstream(listener, reply, seen);
        (address, requests, task)
    }

    fn serve_upstream(
        listener: TcpListener,
        reply: Reply,
        seen: mpsc::UnboundedSender<HeaderMap>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let reply = reply.clone();
                let seen = seen.clone();
                tokio::spawn(async move {
                    let service = service_fn(move |request: Request<Incoming>| {
                        let reply = reply.clone();
                        let seen = seen.clone();
                        async move {
                            let _ = seen.send(request.headers().clone());
                            let mut response = Response::new(Full::new(reply.body));
                            *response.status_mut() = reply.status;
                            response
                                .headers_mut()
                                .insert(CONTENT_TYPE, HeaderValue::from_static(reply.content_type));
                            if let Some(encoding) = reply.content_encoding {
                                response
                                    .headers_mut()
                                    .insert(CONTENT_ENCODING, HeaderValue::from_static(encoding));
                            }
                            Ok::<_, Infallible>(response)
                        }
                    });
                    let _ = server_http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        })
    }

    async fn request(address: SocketAddr, request: Request<Full<Bytes>>) -> Response<Incoming> {
        let stream = TcpStream::connect(address).await.unwrap();
        let (mut sender, connection) = client_http1::handshake(TokioIo::new(stream)).await.unwrap();
        tokio::spawn(async move {
            let _ = connection.await;
        });
        sender.send_request(request).await.unwrap()
    }

    fn config(root: &TempDir, upstream: SocketAddr) -> InjectorConfig {
        let css_file = root.path().join("inject.css");
        std::fs::write(&css_file, "html{color:red}").unwrap();
        InjectorConfig {
            listen: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            upstream,
            css_file,
            font_file: None,
            hold: Duration::ZERO,
        }
    }

    #[test]
    fn generated_css_matches_the_existing_contract() {
        let css = injected_css("#101010", "JetBrains Mono");
        assert!(
            css.contains("@font-face{font-family:\"JetBrains Mono\";src:url(\"/__tode/font.ttf\")")
        );
        assert!(css.contains("html,body{background:#101010 !important;}"));
        assert!(css.contains(".monaco-workbench{background:#101010;"));
        assert!(css.contains("Menlo, \"DejaVu Sans Mono\", \"Liberation Mono\", monospace"));
        assert!(css.contains(".editor-group-watermark{display:none !important;}"));
        assert!(!css.contains("<script"));
    }

    #[tokio::test]
    async fn injects_html_and_rewrites_upstream_headers() {
        let (upstream, mut seen, upstream_task) = start_upstream(Reply {
            status: StatusCode::OK,
            content_type: "text/html",
            body: Bytes::from_static(b"<html><head></head><body>hi</body></html>"),
            content_encoding: Some("gzip"),
        })
        .await;
        let root = TempDir::new().unwrap();
        let injector = Injector::start(config(&root, upstream)).await.unwrap();
        let outbound = Request::builder()
            .uri("/")
            .header(ACCEPT, "text/html")
            .header(ACCEPT_ENCODING, "gzip")
            .header(ORIGIN, "https://outside.example/path")
            .header(REFERER, "https://outside.example/workbench")
            .body(Full::new(Bytes::new()))
            .unwrap();
        let response = request(injector.address(), outbound).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get(CONTENT_ENCODING).is_none());
        assert!(response.headers().get(TRANSFER_ENCODING).is_none());
        let declared = response.headers()[CONTENT_LENGTH]
            .to_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(declared, body.len());
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("<style id=\"tode-injected\">html{color:red}</style></head>"));
        let headers = seen.recv().await.unwrap();
        assert_eq!(headers[HOST], upstream.to_string());
        assert_eq!(headers[ACCEPT_ENCODING], "identity");
        assert_eq!(headers[ORIGIN], format!("http://{upstream}/path"));
        assert_eq!(headers[REFERER], format!("http://{upstream}/workbench"));
        injector.shutdown().await;
        upstream_task.abort();
    }

    #[tokio::test]
    async fn prepends_style_when_html_has_no_head() {
        let (upstream, _seen, upstream_task) = start_upstream(Reply {
            status: StatusCode::OK,
            content_type: "text/html",
            body: Bytes::from_static(b"<body>hi</body>"),
            content_encoding: None,
        })
        .await;
        let root = TempDir::new().unwrap();
        let injector = Injector::start(config(&root, upstream)).await.unwrap();
        let response = request(
            injector.address(),
            Request::builder()
                .uri("/")
                .header(ACCEPT, "text/html")
                .body(Full::new(Bytes::new()))
                .unwrap(),
        )
        .await;
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.starts_with(b"<style id=\"tode-injected\">"));
        injector.shutdown().await;
        upstream_task.abort();
    }

    #[tokio::test]
    async fn passes_non_html_and_html_without_css_unchanged() {
        for (content_type, remove_css) in [("application/json", false), ("text/html", true)] {
            let payload = Bytes::from_static(b"{\"hello\":\"world\"}");
            let (upstream, _seen, upstream_task) = start_upstream(Reply {
                status: StatusCode::CREATED,
                content_type,
                body: payload.clone(),
                content_encoding: None,
            })
            .await;
            let root = TempDir::new().unwrap();
            let configuration = config(&root, upstream);
            if remove_css {
                std::fs::remove_file(&configuration.css_file).unwrap();
            }
            let injector = Injector::start(configuration).await.unwrap();
            let response = request(
                injector.address(),
                Request::builder()
                    .uri("/")
                    .header(ACCEPT, "text/html")
                    .body(Full::new(Bytes::new()))
                    .unwrap(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::CREATED);
            assert_eq!(
                response.into_body().collect().await.unwrap().to_bytes(),
                payload
            );
            injector.shutdown().await;
            upstream_task.abort();
        }
    }

    #[tokio::test]
    async fn serves_font_and_returns_controlled_upstream_error() {
        let root = TempDir::new().unwrap();
        let css = root.path().join("inject.css");
        std::fs::write(&css, "").unwrap();
        let font = root.path().join("font.ttf");
        std::fs::write(&font, [0_u8, 1, 2, 3]).unwrap();
        let unused = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let upstream = unused.local_addr().unwrap();
        drop(unused);
        let injector = Injector::start(InjectorConfig {
            listen: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            upstream,
            css_file: css,
            font_file: Some(font),
            hold: Duration::ZERO,
        })
        .await
        .unwrap();
        let response = request(
            injector.address(),
            Request::builder()
                .uri(FONT_ROUTE)
                .body(Full::new(Bytes::new()))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "font/ttf");
        assert_eq!(
            response.headers()[CACHE_CONTROL],
            "public, max-age=31536000, immutable"
        );
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            Bytes::from_static(&[0, 1, 2, 3])
        );

        let response = request(
            injector.address(),
            Request::builder()
                .uri("/")
                .body(Full::new(Bytes::new()))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            Bytes::from_static(b"tode: code-server is not answering\n")
        );
        injector.shutdown().await;
    }

    #[tokio::test]
    async fn waits_for_initial_upstream_readiness() {
        let reservation = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let upstream = reservation.local_addr().unwrap();
        drop(reservation);
        let root = TempDir::new().unwrap();
        let mut configuration = config(&root, upstream);
        configuration.hold = Duration::from_secs(1);
        let injector = Injector::start(configuration).await.unwrap();
        let address = injector.address();
        let pending = tokio::spawn(async move {
            request(
                address,
                Request::builder()
                    .uri("/")
                    .body(Full::new(Bytes::new()))
                    .unwrap(),
            )
            .await
        });
        tokio::time::sleep(Duration::from_millis(80)).await;
        let listener = TcpListener::bind(upstream).await.unwrap();
        let (seen_tx, _seen_rx) = mpsc::unbounded_channel();
        let upstream_task = serve_upstream(
            listener,
            Reply {
                status: StatusCode::OK,
                content_type: "text/plain",
                body: Bytes::from_static(b"ready"),
                content_encoding: None,
            },
            seen_tx,
        );
        let response = pending.await.unwrap();
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            Bytes::from_static(b"ready")
        );
        injector.shutdown().await;
        upstream_task.abort();
    }

    #[tokio::test]
    async fn bridges_http_upgrade_and_buffered_head_bytes() {
        let upstream_listener =
            TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
                .await
                .unwrap();
        let upstream = upstream_listener.local_addr().unwrap();
        let upstream_task = tokio::spawn(async move {
            let (mut stream, _) = upstream_listener.accept().await.unwrap();
            let mut handshake = Vec::new();
            let mut chunk = [0_u8; 1024];
            while !handshake.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut chunk).await.unwrap();
                assert!(read > 0);
                handshake.extend_from_slice(&chunk[..read]);
            }
            assert!(
                String::from_utf8_lossy(&handshake)
                    .to_ascii_lowercase()
                    .contains("upgrade: test")
            );
            stream
                .write_all(
                    b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: test\r\n\r\nupstream-head",
                )
                .await
                .unwrap();
            let mut client_head = vec![0_u8; b"client-head".len()];
            tokio::time::timeout(Duration::from_secs(1), stream.read_exact(&mut client_head))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(client_head, b"client-head");
        });

        let root = TempDir::new().unwrap();
        let injector = Injector::start(config(&root, upstream)).await.unwrap();
        let mut client = TcpStream::connect(injector.address()).await.unwrap();
        client
            .write_all(
                b"GET /socket HTTP/1.1\r\nHost: client\r\nConnection: Upgrade\r\nUpgrade: test\r\n\r\nclient-head",
            )
            .await
            .unwrap();
        let mut received = Vec::new();
        let mut chunk = [0_u8; 1024];
        tokio::time::timeout(Duration::from_secs(1), async {
            while !received
                .windows(b"upstream-head".len())
                .any(|window| window == b"upstream-head")
            {
                let read = client.read(&mut chunk).await.unwrap();
                assert!(read > 0);
                received.extend_from_slice(&chunk[..read]);
            }
        })
        .await
        .unwrap();
        assert!(String::from_utf8_lossy(&received).contains("101 Switching Protocols"));
        drop(client);
        upstream_task.await.unwrap();
        injector.shutdown().await;
    }
}
