use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn sends_goto_wait_and_review_to_existing_window() {
    let root = TempDir::new().unwrap();
    let socket = root.path().join("window.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let (sent, received) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut connection, _) = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(connection.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        sent.send(line).unwrap();
        connection.write_all(b"{\"ok\":true}\n").unwrap();
    });
    let output = Command::new(env!("CARGO_BIN_EXE_tode"))
        .current_dir(root.path())
        .env("HOME", root.path().join("home"))
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env("XDG_CACHE_HOME", root.path().join("cache"))
        .env("TODE_IPC", &socket)
        .args(["--goto", "--wait", "--review", "src/main.rs:12:4"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    server.join().unwrap();
    let request: Value = serde_json::from_str(received.recv().unwrap().trim()).unwrap();
    assert_eq!(request["files"][0]["line"], 12);
    assert_eq!(request["files"][0]["column"], 4);
    assert!(
        request["files"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("src/main.rs")
    );
    assert_eq!(request["folders"], serde_json::json!([]));
    assert_eq!(request["add"], false);
    assert_eq!(request["wait"], true);
    assert_eq!(request["view"], "scm");
}
