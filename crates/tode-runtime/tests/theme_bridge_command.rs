use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

use tempfile::TempDir;

#[test]
fn generates_theme_and_rejects_missing_required_colors() {
    let root = TempDir::new().unwrap();
    let sockets = root.path().join("sockets");
    fs::create_dir(&sockets).unwrap();
    let listener = UnixListener::bind(sockets.join("window.sock")).unwrap();
    let received = thread::spawn(move || {
        let (mut connection, _) = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(connection.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        connection.write_all(b"{\"ok\":true}\n").unwrap();
        serde_json::from_str::<serde_json::Value>(line.trim()).unwrap()
    });
    let valid = run(
        r#"{"background":[1,2,3],"foreground":[240,241,242],"ansi":[]}"#,
        Some(&sockets),
    );
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let theme: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(theme["colors"]["editor.background"], "#010203");
    assert!(theme["tokenColors"].as_array().unwrap().len() > 10);
    let request = received.join().unwrap();
    assert_eq!(request["theme"]["colors"]["editor.background"], "#010203");

    let invalid = run(r#"{"foreground":[1,2,3]}"#, None);
    assert_eq!(invalid.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("terminal theme has no background"));
}

fn run(input: &str, socket_dir: Option<&Path>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tode-theme-bridge"));
    if let Some(socket_dir) = socket_dir {
        command.arg("--socket-dir").arg(socket_dir);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
