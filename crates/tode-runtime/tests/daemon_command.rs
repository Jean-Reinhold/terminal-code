use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::process::{Command, Stdio};
use std::time::Duration;

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use tempfile::TempDir;
use tode_runtime::{ServerState, current_server, pid_running};

#[test]
fn announces_readiness_and_cleans_up_on_sigterm() {
    let reservation =
        TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let root = TempDir::new().unwrap();
    let css = root.path().join("inject.css");
    std::fs::write(&css, "html{color:red}").unwrap();
    let state_file = root.path().join("state/server.json");
    let mut daemon = Command::new(env!("CARGO_BIN_EXE_tode-daemon"))
        .args([
            "--code-server",
            env!("CARGO_BIN_EXE_tode-code-server-probe"),
            "--code-port",
            &port.to_string(),
            "--user-data",
            root.path().join("user").to_str().unwrap(),
            "--extensions",
            root.path().join("extensions").to_str().unwrap(),
            "--log",
            root.path().join("logs/code-server.log").to_str().unwrap(),
            "--css",
            css.to_str().unwrap(),
            "--state",
            state_file.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(daemon.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let state: ServerState = serde_json::from_str(line.trim()).unwrap();
    assert!(pid_running(state.pid));
    assert_eq!(state.injector_pid, daemon.id() as i32);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    assert_eq!(
        runtime.block_on(current_server(&state_file, Duration::from_millis(100))),
        Some(state.clone())
    );
    kill(Pid::from_raw(daemon.id() as i32), Signal::SIGTERM).unwrap();
    assert!(daemon.wait().unwrap().success());
    assert!(!state_file.exists());
    assert!(!pid_running(state.pid));
}
