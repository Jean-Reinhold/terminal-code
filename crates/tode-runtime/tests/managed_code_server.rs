use std::ffi::OsString;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::path::Path;
use std::time::Duration;

use tempfile::TempDir;
use tode_runtime::{
    CodeServerConfig, answering, code_server_arguments, extensions_gallery, pid_running,
    start_code_server,
};

#[test]
fn arguments_and_gallery_match_the_legacy_contract() {
    let arguments =
        code_server_arguments(4321, Path::new("/profile/user"), Path::new("/profile/ext"));
    assert_eq!(
        arguments,
        vec![
            OsString::from("--auth"),
            OsString::from("none"),
            OsString::from("--bind-addr"),
            OsString::from("127.0.0.1:4321"),
            OsString::from("--user-data-dir"),
            OsString::from("/profile/user"),
            OsString::from("--extensions-dir"),
            OsString::from("/profile/ext"),
            OsString::from("--app-name"),
            OsString::from("tode"),
            OsString::from("--disable-telemetry"),
            OsString::from("--disable-update-check"),
            OsString::from("--disable-workspace-trust"),
            OsString::from("--disable-getting-started-override"),
            OsString::from("--ignore-last-opened"),
        ]
    );
    let gallery: serde_json::Value = serde_json::from_str(extensions_gallery()).unwrap();
    assert_eq!(
        gallery["serviceUrl"],
        "https://marketplace.visualstudio.com/_apis/public/gallery"
    );
    assert_eq!(gallery["controlUrl"], "");
}

#[tokio::test]
async fn starts_waits_and_stops_managed_code_server() {
    let reservation =
        TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let root = TempDir::new().unwrap();
    let managed = start_code_server(&CodeServerConfig {
        binary: env!("CARGO_BIN_EXE_tode-code-server-probe").into(),
        port,
        user_data: root.path().join("user"),
        extensions: root.path().join("extensions"),
        log_file: root.path().join("logs/code-server.log"),
        readiness_deadline: Duration::from_secs(2),
    })
    .await
    .unwrap();
    assert_eq!(managed.version, "probe-code-server 1.0");
    assert!(pid_running(managed.pid));
    assert!(answering(port, Duration::from_millis(100)).await);
    assert!(root.path().join("logs/code-server.log").is_file());
    let pid = managed.pid;
    managed.shutdown().unwrap();
    assert!(!pid_running(pid));
}
