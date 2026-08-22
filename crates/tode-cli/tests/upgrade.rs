use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;

use flate2::Compression;
use flate2::write::GzEncoder;
use sha2::{Digest, Sha256};
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

#[test]
fn upgrades_verified_tree_reports_transition_and_stops_state() {
    let root = TempDir::new().unwrap();
    let install = root.path().join("install");
    std::fs::create_dir(&install).unwrap();
    std::fs::write(install.join("VERSION"), "v1\n").unwrap();
    std::fs::write(install.join("CHANNEL"), "stable\n").unwrap();
    std::fs::write(install.join("old-file"), "old\n").unwrap();
    let state = root.path().join("state/tode/server.json");
    std::fs::create_dir_all(state.parent().unwrap()).unwrap();
    std::fs::write(
        &state,
        serde_json::to_vec(&serde_json::json!({
            "pid": 2_000_000_000,
            "port": 1,
            "injectorPid": 2_000_000_001_i64,
            "injectorPort": 2,
            "version": "test",
            "startedAt": 1
        }))
        .unwrap(),
    )
    .unwrap();
    let archive = release_archive();
    let digest = hex::encode(Sha256::digest(&archive));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let platform = tode_core::current_target_triple();
    let manifest = serde_json::to_vec(&serde_json::json!({
        "version": "v2",
        "channel": "stable",
        "platforms": {
            platform: {
                "file": "tode.tar.gz",
                "sha256": digest,
                "size": archive.len(),
                "url": format!("http://{address}/artifact")
            }
        }
    }))
    .unwrap();
    let server = serve(listener, vec![manifest, archive]);
    let output = Command::new(env!("CARGO_BIN_EXE_tode"))
        .env("HOME", root.path().join("home"))
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env("XDG_CACHE_HOME", root.path().join("cache"))
        .env("TODE_INSTALL_ROOT", &install)
        .env("TODE_RELEASE_ORIGIN", format!("http://{address}"))
        .arg("--upgrade")
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tode v1 -> v2\n");
    assert_eq!(
        std::fs::read_to_string(install.join("VERSION")).unwrap(),
        "v2\n"
    );
    assert_eq!(
        std::fs::read_to_string(install.join("CHANNEL")).unwrap(),
        "stable\n"
    );
    assert_eq!(
        std::fs::read_to_string(install.join("new-file")).unwrap(),
        "new\n"
    );
    assert!(!install.join("old-file").exists());
    assert!(!state.exists());
}

fn release_archive() -> Vec<u8> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut archive = tar::Builder::new(encoder);
    let data = b"new\n";
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    archive
        .append_data(&mut header, "bundle/new-file", &data[..])
        .unwrap();
    archive.into_inner().unwrap().finish().unwrap()
}

fn serve(listener: TcpListener, bodies: Vec<Vec<u8>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for body in bodies {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .unwrap();
            stream.write_all(&body).unwrap();
        }
    })
}
