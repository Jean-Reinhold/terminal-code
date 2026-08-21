use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("artifact I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("artifact size expected {expected}, got {actual}")]
    Size { expected: u64, actual: u64 },
    #[error("artifact SHA-256 expected {expected}, got {actual}")]
    Sha256 { expected: String, actual: String },
    #[error("unsafe archive entry: {0}")]
    UnsafeArchive(String),
    #[error("archive exceeds limit: {0}")]
    ArchiveLimit(String),
}

pub async fn download_verified(
    client: &Client,
    url: &str,
    expected_sha256: &str,
    expected_size: u64,
    destination: &Path,
) -> Result<(), ArtifactError> {
    let parent = destination.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination has no parent",
        )
    })?;
    fs::create_dir_all(parent)?;
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    let (standard, temporary_path) = temporary.keep().map_err(|error| error.error)?;
    let mut file = tokio::fs::File::from_std(standard);
    let result = async {
        let mut response = client.get(url).send().await?.error_for_status()?;
        let mut hasher = Sha256::new();
        let mut size = 0_u64;
        while let Some(chunk) = response.chunk().await? {
            size = size.saturating_add(chunk.len() as u64);
            if size > expected_size {
                return Err(ArtifactError::Size {
                    expected: expected_size,
                    actual: size,
                });
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }
        if size != expected_size {
            return Err(ArtifactError::Size {
                expected: expected_size,
                actual: size,
            });
        }
        let actual = hex::encode(hasher.finalize());
        if actual != expected_sha256 {
            return Err(ArtifactError::Sha256 {
                expected: expected_sha256.into(),
                actual,
            });
        }
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        tokio::fs::rename(&temporary_path, destination).await?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&temporary_path).await;
    }
    result
}

pub fn unpack_tar_gz(
    archive_path: &Path,
    destination: &Path,
    max_entries: usize,
    max_bytes: u64,
) -> Result<(), ArtifactError> {
    unpack_tar_gz_stripped(archive_path, destination, 0, max_entries, max_bytes)
}

pub fn unpack_tar_gz_stripped(
    archive_path: &Path,
    destination: &Path,
    strip_components: usize,
    max_entries: usize,
    max_bytes: u64,
) -> Result<(), ArtifactError> {
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::create_dir_all(destination)?;
    let result = unpack_inner(
        archive_path,
        destination,
        strip_components,
        max_entries,
        max_bytes,
    );
    if result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    result
}

pub fn swap_directory(staged: &Path, live: &Path) -> Result<(), ArtifactError> {
    let name = live
        .file_name()
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "live path has no name")
        })?
        .to_string_lossy();
    let old = live.with_file_name(format!("{name}.old"));
    if old.exists() {
        fs::remove_dir_all(&old)?;
    }
    let had_live = live.exists();
    if had_live {
        fs::rename(live, &old)?;
    }
    if let Err(error) = fs::rename(staged, live) {
        if had_live {
            let _ = fs::rename(&old, live);
        }
        return Err(ArtifactError::Io(error));
    }
    if old.exists() {
        fs::remove_dir_all(old)?;
    }
    Ok(())
}

fn unpack_inner(
    archive_path: &Path,
    destination: &Path,
    strip_components: usize,
    max_entries: usize,
    max_bytes: u64,
) -> Result<(), ArtifactError> {
    let archive = fs::File::open(archive_path)?;
    let mut archive = tar::Archive::new(GzDecoder::new(archive));
    let mut entries = 0_usize;
    let mut bytes = 0_u64;
    for entry in archive.entries()? {
        let mut entry = entry?;
        entries += 1;
        if entries > max_entries {
            return Err(ArtifactError::ArchiveLimit(format!(
                "more than {max_entries} entries"
            )));
        }
        let path = entry.path()?.into_owned();
        if path.as_os_str().is_empty()
            || path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(ArtifactError::UnsafeArchive(path.display().to_string()));
        }
        let path: PathBuf = path.components().skip(strip_components).collect();
        if path.as_os_str().is_empty() {
            continue;
        }
        let entry_type = entry.header().entry_type();
        if !(entry_type.is_dir() || entry_type.is_file()) {
            return Err(ArtifactError::UnsafeArchive(format!(
                "{} has unsupported type",
                path.display()
            )));
        }
        let destination_path = destination.join(&path);
        if entry_type.is_dir() {
            fs::create_dir_all(&destination_path)?;
            continue;
        }
        let size = entry.header().size()?;
        bytes = bytes.saturating_add(size);
        if bytes > max_bytes {
            return Err(ArtifactError::ArchiveLimit(format!(
                "expanded bytes exceed {max_bytes}"
            )));
        }
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination_path)?;
        std::io::copy(&mut entry, &mut output)?;
        output.flush()?;
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String, ArtifactError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::path::PathBuf;

    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    #[tokio::test]
    async fn downloads_only_exact_size_and_hash() {
        let body = b"verified artifact";
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
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
            stream.write_all(body).await.unwrap();
        });
        let root = TempDir::new().unwrap();
        let destination = root.path().join("artifact.bin");
        let digest = hex::encode(Sha256::digest(body));
        download_verified(
            &Client::new(),
            &format!("http://{address}/artifact"),
            &digest,
            body.len() as u64,
            &destination,
        )
        .await
        .unwrap();
        server.await.unwrap();
        assert_eq!(fs::read(destination).unwrap(), body);
    }

    #[tokio::test]
    async fn failed_verification_leaves_no_destination() {
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await.unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nbad!")
                .await
                .unwrap();
        });
        let root = TempDir::new().unwrap();
        let destination = root.path().join("artifact.bin");
        assert!(
            download_verified(
                &Client::new(),
                &format!("http://{address}/artifact"),
                &"0".repeat(64),
                4,
                &destination,
            )
            .await
            .is_err()
        );
        assert!(!destination.exists());
    }

    #[test]
    fn safely_unpacks_regular_files_and_enforces_limits() {
        let root = TempDir::new().unwrap();
        let archive = root.path().join("good.tar.gz");
        write_archive(&archive, false);
        let destination = root.path().join("out");
        unpack_tar_gz(&archive, &destination, 10, 1024).unwrap();
        assert_eq!(
            fs::read(destination.join("folder/file.txt")).unwrap(),
            b"hello"
        );
        assert!(unpack_tar_gz(&archive, &destination, 0, 1024).is_err());
        assert!(!destination.exists());
        assert!(unpack_tar_gz(&archive, &destination, 10, 2).is_err());
        assert!(!destination.exists());
    }

    #[test]
    fn rejects_link_entries() {
        let root = TempDir::new().unwrap();
        let archive = root.path().join("link.tar.gz");
        write_archive(&archive, true);
        let destination = root.path().join("out");
        assert!(matches!(
            unpack_tar_gz(&archive, &destination, 10, 1024),
            Err(ArtifactError::UnsafeArchive(_))
        ));
        assert!(!destination.exists());
    }

    #[test]
    fn failed_swap_restores_previous_install() {
        let root = TempDir::new().unwrap();
        let live = root.path().join("tode");
        fs::create_dir(&live).unwrap();
        fs::write(live.join("VERSION"), "old").unwrap();
        assert!(swap_directory(&root.path().join("missing"), &live).is_err());
        assert_eq!(fs::read_to_string(live.join("VERSION")).unwrap(), "old");

        let staged = root.path().join("staged");
        fs::create_dir(&staged).unwrap();
        fs::write(staged.join("VERSION"), "new").unwrap();
        swap_directory(&staged, &live).unwrap();
        assert_eq!(fs::read_to_string(live.join("VERSION")).unwrap(), "new");
        assert!(!root.path().join("tode.old").exists());
    }

    fn write_archive(path: &Path, link: bool) {
        let file = fs::File::create(path).unwrap();
        let encoder = GzEncoder::new(file, Compression::default());
        let mut archive = tar::Builder::new(encoder);
        if link {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_mode(0o777);
            header.set_cksum();
            archive
                .append_link(&mut header, "link", PathBuf::from("../outside"))
                .unwrap();
        } else {
            let data = b"hello";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive
                .append_data(&mut header, "folder/file.txt", &data[..])
                .unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap();
    }
}
