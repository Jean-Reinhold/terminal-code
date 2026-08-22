use std::fs;
use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tode_core::{Build, installed_receipt};

use crate::{ArtifactError, download_verified, swap_directory, unpack_tar_gz_stripped};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum UpgradeOutcome {
    Current { version: String, channel: String },
    Available { from: String, build: Build },
    Upgraded { from: String, build: Build },
}

#[derive(Debug, thiserror::Error)]
pub enum UpgradeError {
    #[error(transparent)]
    Artifact(#[from] ArtifactError),
    #[error("upgrade I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn apply_build(
    client: &Client,
    build: &Build,
    install_root: &Path,
    check: bool,
) -> Result<UpgradeOutcome, UpgradeError> {
    let installed = installed_receipt(install_root);
    if installed.as_ref().is_some_and(|installed| {
        installed.version == build.version && installed.channel == build.channel
    }) {
        return Ok(UpgradeOutcome::Current {
            version: build.version.clone(),
            channel: build.channel.clone(),
        });
    }
    let from = installed
        .map(|installed| installed.version)
        .unwrap_or_else(|| "unknown".into());
    if check {
        return Ok(UpgradeOutcome::Available {
            from,
            build: build.clone(),
        });
    }
    let tarball = sibling(install_root, "download.tar.gz")?;
    let staged = sibling(install_root, "new")?;
    download_verified(client, &build.url, &build.sha256, build.size, &tarball).await?;
    let unpacked = unpack_tar_gz_stripped(&tarball, &staged, 1, 300_000, 4 * 1024 * 1024 * 1024);
    let _ = fs::remove_file(&tarball);
    unpacked?;
    fs::write(staged.join("VERSION"), format!("{}\n", build.version))?;
    fs::write(staged.join("CHANNEL"), format!("{}\n", build.channel))?;
    swap_directory(&staged, install_root)?;
    Ok(UpgradeOutcome::Upgraded {
        from,
        build: build.clone(),
    })
}

fn sibling(path: &Path, suffix: &str) -> std::io::Result<PathBuf> {
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "install root has no name")
    })?;
    Ok(path.with_file_name(format!("{}.{}", name.to_string_lossy(), suffix)))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use flate2::Compression;
    use flate2::write::GzEncoder;
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    #[tokio::test]
    async fn reports_current_and_available_without_download() {
        let root = TempDir::new().unwrap();
        fs::write(root.path().join("VERSION"), "v1\n").unwrap();
        fs::write(root.path().join("CHANNEL"), "stable\n").unwrap();
        let current_build = build("v1", "stable", "http://unused", "0", 0);
        assert!(matches!(
            apply_build(&Client::new(), &current_build, root.path(), false)
                .await
                .unwrap(),
            UpgradeOutcome::Current { .. }
        ));
        let next = build("v2", "stable", "http://unused", "0", 0);
        assert!(matches!(
            apply_build(&Client::new(), &next, root.path(), true)
                .await
                .unwrap(),
            UpgradeOutcome::Available { from, .. } if from == "v1"
        ));
    }

    #[tokio::test]
    async fn downloads_stages_receipts_and_swaps_complete_tree() {
        let install = TempDir::new().unwrap();
        fs::write(install.path().join("VERSION"), "v1\n").unwrap();
        fs::write(install.path().join("CHANNEL"), "stable\n").unwrap();
        fs::write(install.path().join("old"), "old").unwrap();
        let archive = release_archive();
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_once(listener, archive.clone());
        let build = build(
            "v2",
            "stable",
            &format!("http://{address}/artifact"),
            &hex::encode(Sha256::digest(&archive)),
            archive.len() as u64,
        );
        let outcome = apply_build(&Client::new(), &build, install.path(), false)
            .await
            .unwrap();
        server.await.unwrap();
        assert!(matches!(outcome, UpgradeOutcome::Upgraded { from, .. } if from == "v1"));
        assert_eq!(
            fs::read_to_string(install.path().join("VERSION")).unwrap(),
            "v2\n"
        );
        assert_eq!(
            fs::read_to_string(install.path().join("CHANNEL")).unwrap(),
            "stable\n"
        );
        assert!(install.path().join("new-file").is_file());
        assert!(!install.path().join("old").exists());
    }

    #[tokio::test]
    async fn failed_verification_preserves_previous_install() {
        let install = TempDir::new().unwrap();
        fs::write(install.path().join("VERSION"), "v1\n").unwrap();
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_once(listener, b"bad".to_vec());
        let build = build(
            "v2",
            "stable",
            &format!("http://{address}"),
            &"0".repeat(64),
            3,
        );
        assert!(
            apply_build(&Client::new(), &build, install.path(), false)
                .await
                .is_err()
        );
        server.await.unwrap();
        assert_eq!(
            fs::read_to_string(install.path().join("VERSION")).unwrap(),
            "v1\n"
        );
    }

    fn build(version: &str, channel: &str, url: &str, sha256: &str, size: u64) -> Build {
        Build {
            version: version.into(),
            channel: channel.into(),
            platform: "test".into(),
            file: "tode.tar.gz".into(),
            sha256: sha256.into(),
            size,
            url: url.into(),
        }
    }

    fn release_archive() -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let data = b"new";
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(&mut header, "bundle/new-file", &data[..])
            .unwrap();
        archive.into_inner().unwrap().finish().unwrap()
    }

    fn serve_once(listener: TcpListener, body: Vec<u8>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await.unwrap();
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            stream.write_all(&body).await.unwrap();
        })
    }
}
