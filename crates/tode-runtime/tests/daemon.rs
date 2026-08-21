use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::time::Duration;

use tempfile::TempDir;
use tode_runtime::{CodeServerConfig, Daemon, DaemonConfig, current_server, pid_running};

#[tokio::test]
async fn composes_code_server_injector_state_and_shutdown() {
    let reservation =
        TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).unwrap();
    let code_port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let root = TempDir::new().unwrap();
    let css = root.path().join("inject.css");
    std::fs::write(&css, "html{color:red}").unwrap();
    let state_file = root.path().join("state/server.json");
    let daemon = Daemon::start(DaemonConfig {
        code_server: CodeServerConfig {
            binary: env!("CARGO_BIN_EXE_tode-code-server-probe").into(),
            port: code_port,
            user_data: root.path().join("user"),
            extensions: root.path().join("extensions"),
            log_file: root.path().join("logs/code-server.log"),
            readiness_deadline: Duration::from_secs(2),
        },
        injector_listen: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        css_file: css,
        font_file: None,
        injector_hold: Duration::from_secs(1),
        state_file: state_file.clone(),
    })
    .await
    .unwrap();
    let code_pid = daemon.state.pid;
    assert!(pid_running(code_pid));
    assert_eq!(
        current_server(&state_file, Duration::from_millis(100)).await,
        Some(daemon.state.clone())
    );
    let response = reqwest::get(daemon.origin()).await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "ok");
    daemon.shutdown().await.unwrap();
    assert!(!state_file.exists());
    assert!(!pid_running(code_pid));
}
