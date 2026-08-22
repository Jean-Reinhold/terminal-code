use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;

use tempfile::TempDir;

#[test]
fn check_reports_available_verified_manifest_build() {
    let root = TempDir::new().unwrap();
    let install = root.path().join("install");
    std::fs::create_dir(&install).unwrap();
    std::fs::write(install.join("VERSION"), "v1\n").unwrap();
    std::fs::write(install.join("CHANNEL"), "stable\n").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let platform = tode_core::current_target_triple();
    let manifest = serde_json::to_vec(&serde_json::json!({
        "version": "v2",
        "channel": "stable",
        "platforms": {
            platform: {
                "file": "tode.tar.gz",
                "sha256": "unused",
                "size": 0,
                "url": "http://unused"
            }
        }
    }))
    .unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 2048];
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                    manifest.len()
                )
                .as_bytes(),
            )
            .unwrap();
        stream.write_all(&manifest).unwrap();
    });
    let output = Command::new(env!("CARGO_BIN_EXE_tode"))
        .env("HOME", root.path().join("home"))
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env("XDG_CACHE_HOME", root.path().join("cache"))
        .env("TODE_INSTALL_ROOT", &install)
        .env("TODE_RELEASE_ORIGIN", format!("http://{address}"))
        .args(["--upgrade", "--check"])
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "tode v2 is available (you have v1)\n"
    );
}
