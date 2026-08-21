use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{HarnessError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub sha256: String,
    pub bytes: u64,
    pub media_type: String,
}

#[derive(Debug, Clone)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("objects/sha256")).map_err(|error| {
            HarnessError::io(format!("create artifact root {}", root.display()), error)
        })?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn put(&self, bytes: &[u8], media_type: impl Into<String>) -> Result<ArtifactRef> {
        let digest = hex::encode(Sha256::digest(bytes));
        let destination = self.object_path(&digest)?;
        let parent = destination.parent().expect("object path has a parent");
        fs::create_dir_all(parent).map_err(|error| {
            HarnessError::io(
                format!("create object directory {}", parent.display()),
                error,
            )
        })?;

        if destination.exists() {
            verify_file(&destination, &digest, bytes.len() as u64)?;
        } else {
            let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
                HarnessError::io(
                    format!("create temporary object in {}", parent.display()),
                    error,
                )
            })?;
            temporary
                .write_all(bytes)
                .map_err(|error| HarnessError::io("write temporary artifact", error))?;
            temporary
                .as_file()
                .sync_all()
                .map_err(|error| HarnessError::io("sync temporary artifact", error))?;
            match temporary.persist_noclobber(&destination) {
                Ok(_) => {}
                Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                    verify_file(&destination, &digest, bytes.len() as u64)?;
                }
                Err(error) => {
                    return Err(HarnessError::io(
                        format!("persist artifact {}", destination.display()),
                        error.error,
                    ));
                }
            }
        }

        Ok(ArtifactRef {
            sha256: digest,
            bytes: bytes.len() as u64,
            media_type: media_type.into(),
        })
    }

    pub fn get(&self, reference: &ArtifactRef) -> Result<Vec<u8>> {
        let path = self.object_path(&reference.sha256)?;
        let bytes = fs::read(&path).map_err(|error| {
            HarnessError::io(format!("read artifact {}", path.display()), error)
        })?;
        let actual = hex::encode(Sha256::digest(&bytes));
        if actual != reference.sha256 || bytes.len() as u64 != reference.bytes {
            return Err(HarnessError::Integrity(format!(
                "artifact {} expected {} bytes {}, got {} bytes {}",
                path.display(),
                reference.sha256,
                reference.bytes,
                actual,
                bytes.len()
            )));
        }
        Ok(bytes)
    }

    pub fn object_path(&self, digest: &str) -> Result<PathBuf> {
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(HarnessError::Integrity(format!(
                "invalid SHA-256 digest {digest}"
            )));
        }
        Ok(self
            .root
            .join("objects/sha256")
            .join(&digest[..2])
            .join(digest))
    }
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|error| HarnessError::Json(error.to_string()))?;
    bytes.push(b'\n');
    let parent = path.parent().ok_or_else(|| {
        HarnessError::Invalid(format!("JSON path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        HarnessError::io(format!("create JSON directory {}", parent.display()), error)
    })?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        HarnessError::io(
            format!("create temporary JSON in {}", parent.display()),
            error,
        )
    })?;
    temporary.write_all(&bytes).map_err(|error| {
        HarnessError::io(
            format!("write temporary JSON for {}", path.display()),
            error,
        )
    })?;
    temporary.as_file().sync_all().map_err(|error| {
        HarnessError::io(format!("sync temporary JSON for {}", path.display()), error)
    })?;
    temporary.persist(path).map_err(|error| {
        HarnessError::io(format!("persist JSON {}", path.display()), error.error)
    })?;
    Ok(())
}

fn verify_file(path: &Path, expected_digest: &str, expected_bytes: u64) -> Result<()> {
    let bytes = fs::read(path).map_err(|error| {
        HarnessError::io(format!("read existing object {}", path.display()), error)
    })?;
    let actual = hex::encode(Sha256::digest(&bytes));
    if actual != expected_digest || bytes.len() as u64 != expected_bytes {
        return Err(HarnessError::Integrity(format!(
            "existing object {} does not match its content address",
            path.display()
        )));
    }
    Ok(())
}
