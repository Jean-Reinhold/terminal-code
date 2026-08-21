use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::artifact::{ArtifactRef, ArtifactStore};
use crate::error::{HarnessError, Result};
use crate::scenario::{EnvironmentValue, ScenarioValue};

const PROTECTED_ENV: &[&str] = &[
    "HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "XDG_CACHE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_BIN_HOME",
    "TODE_INSTALL_ROOT",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FilesystemSnapshot {
    schema_version: u32,
    entries: Vec<FilesystemEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FilesystemEntry {
    path: String,
    kind: String,
    mode: u32,
    content: Option<ArtifactRef>,
}
#[derive(Debug)]
pub struct Sandbox {
    directory: TempDir,
    root: PathBuf,
}

impl Sandbox {
    pub fn create(repo_root: &Path, fixture: Option<&str>) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("tode-harness-")
            .tempdir()
            .map_err(|error| HarnessError::io("create harness sandbox", error))?;
        let root = directory
            .path()
            .canonicalize()
            .map_err(|error| HarnessError::io("canonicalize harness sandbox", error))?;
        if let Some(home) = env::var_os("HOME") {
            let home = PathBuf::from(home);
            if let Ok(home) = home.canonicalize()
                && root.starts_with(&home)
            {
                return Err(HarnessError::Invalid(format!(
                    "sandbox {} is inside real HOME {}",
                    root.display(),
                    home.display()
                )));
            }
        }

        for path in [
            "home",
            "workspace",
            "install",
            "xdg/data",
            "xdg/state",
            "xdg/cache",
            "xdg/config",
            "xdg/bin",
            "logs",
        ] {
            fs::create_dir_all(root.join(path))
                .map_err(|error| HarnessError::io(format!("create sandbox path {path}"), error))?;
        }

        let sandbox = Self { directory, root };
        if let Some(fixture) = fixture {
            sandbox.copy_fixture(repo_root, fixture)?;
        }
        Ok(sandbox)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, relative: &str) -> Result<PathBuf> {
        validate_relative_components(relative)?;
        Ok(self.root.join(relative))
    }

    pub fn environment(
        &self,
        additional: &BTreeMap<String, EnvironmentValue>,
    ) -> Result<BTreeMap<String, String>> {
        let mut environment = BTreeMap::new();
        for (key, value) in additional {
            validate_environment_key(key)?;
            if PROTECTED_ENV.contains(&key.as_str()) {
                return Err(HarnessError::Invalid(format!(
                    "scenario cannot override protected environment variable {key}"
                )));
            }
            let value = match value {
                EnvironmentValue::Literal(value) => value.clone(),
                EnvironmentValue::SandboxPath { sandbox_path } => {
                    self.path(sandbox_path)?.to_string_lossy().into_owned()
                }
            };
            environment.insert(key.clone(), value);
        }

        for (key, relative) in [
            ("HOME", "home"),
            ("XDG_DATA_HOME", "xdg/data"),
            ("XDG_STATE_HOME", "xdg/state"),
            ("XDG_CACHE_HOME", "xdg/cache"),
            ("XDG_CONFIG_HOME", "xdg/config"),
            ("XDG_BIN_HOME", "xdg/bin"),
            ("TODE_INSTALL_ROOT", "install"),
        ] {
            environment.insert(
                key.to_owned(),
                self.root.join(relative).to_string_lossy().into_owned(),
            );
        }
        Ok(environment)
    }

    pub fn resolve_value(&self, value: &ScenarioValue) -> Result<String> {
        match value {
            ScenarioValue::Literal(value) => Ok(value.clone()),
            ScenarioValue::SandboxPath { sandbox_path } => {
                Ok(self.path(sandbox_path)?.to_string_lossy().into_owned())
            }
        }
    }

    pub fn normalize_text(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let text = std::str::from_utf8(bytes).map_err(|error| {
            HarnessError::Invalid(format!(
                "path.sandbox-root-v1 requires UTF-8 process output: {error}"
            ))
        })?;
        Ok(text
            .replace(self.root.to_string_lossy().as_ref(), "$SANDBOX")
            .into_bytes())
    }

    pub fn log_path(&self, name: &str) -> Result<PathBuf> {
        validate_relative_components(name)?;
        Ok(self.root.join("logs").join(name))
    }

    pub fn snapshot_tree(&self, store: &ArtifactStore) -> Result<ArtifactRef> {
        let mut entries = Vec::new();
        let walker = walkdir::WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                entry.path() == self.root
                    || entry
                        .path()
                        .strip_prefix(&self.root)
                        .ok()
                        .and_then(|relative| relative.components().next())
                        .is_none_or(|component| component.as_os_str() != "logs")
            });
        for entry in walker {
            let entry = entry.map_err(|error| HarnessError::Invalid(error.to_string()))?;
            let relative = entry
                .path()
                .strip_prefix(&self.root)
                .map_err(|error| HarnessError::Invalid(error.to_string()))?;
            if relative.as_os_str().is_empty() {
                continue;
            }
            if entry.file_type().is_symlink() {
                return Err(HarnessError::Invalid(format!(
                    "sandbox snapshot rejects symlink: {}",
                    entry.path().display()
                )));
            }
            let metadata = fs::metadata(entry.path()).map_err(|error| {
                HarnessError::io(
                    format!("read snapshot metadata {}", entry.path().display()),
                    error,
                )
            })?;
            let (kind, content) = if entry.file_type().is_dir() {
                ("directory".to_owned(), None)
            } else if entry.file_type().is_file() {
                let bytes = fs::read(entry.path()).map_err(|error| {
                    HarnessError::io(
                        format!("read snapshot file {}", entry.path().display()),
                        error,
                    )
                })?;
                (
                    "file".to_owned(),
                    Some(store.put(&bytes, "application/octet-stream")?),
                )
            } else {
                return Err(HarnessError::Invalid(format!(
                    "unsupported sandbox snapshot entry: {}",
                    entry.path().display()
                )));
            };
            entries.push(FilesystemEntry {
                path: relative.to_string_lossy().replace('\\', "/"),
                kind,
                mode: metadata.permissions().mode() & 0o7777,
                content,
            });
        }
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        let snapshot = FilesystemSnapshot {
            schema_version: 1,
            entries,
        };
        let bytes =
            serde_json::to_vec(&snapshot).map_err(|error| HarnessError::Json(error.to_string()))?;
        store.put(&bytes, "application/json")
    }

    fn copy_fixture(&self, repo_root: &Path, fixture: &str) -> Result<()> {
        validate_relative_components(fixture)?;
        let repo_root = repo_root.canonicalize().map_err(|error| {
            HarnessError::io(format!("canonicalize {}", repo_root.display()), error)
        })?;
        let source = repo_root
            .join(fixture)
            .canonicalize()
            .map_err(|error| HarnessError::io(format!("canonicalize fixture {fixture}"), error))?;
        if !source.starts_with(&repo_root) || !source.is_dir() {
            return Err(HarnessError::Invalid(format!(
                "fixture must be a repository directory: {fixture}"
            )));
        }

        for entry in walkdir::WalkDir::new(&source).follow_links(false) {
            let entry = entry.map_err(|error| HarnessError::Invalid(error.to_string()))?;
            let relative = entry
                .path()
                .strip_prefix(&source)
                .map_err(|error| HarnessError::Invalid(format!("strip fixture prefix: {error}")))?;
            if relative.as_os_str().is_empty() {
                continue;
            }
            if entry.file_type().is_symlink() {
                return Err(HarnessError::Invalid(format!(
                    "fixture symlinks are not allowed in S1: {}",
                    entry.path().display()
                )));
            }
            let destination = self.root.join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&destination).map_err(|error| {
                    HarnessError::io(
                        format!("create fixture directory {}", destination.display()),
                        error,
                    )
                })?;
            } else if entry.file_type().is_file() {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        HarnessError::io(
                            format!("create fixture parent {}", parent.display()),
                            error,
                        )
                    })?;
                }
                fs::copy(entry.path(), &destination).map_err(|error| {
                    HarnessError::io(
                        format!(
                            "copy fixture {} to {}",
                            entry.path().display(),
                            destination.display()
                        ),
                        error,
                    )
                })?;
                let permissions = fs::metadata(entry.path())
                    .map_err(|error| HarnessError::io("read fixture permissions", error))?
                    .permissions();
                fs::set_permissions(&destination, permissions).map_err(|error| {
                    HarnessError::io(
                        format!("set fixture permissions {}", destination.display()),
                        error,
                    )
                })?;
            } else {
                return Err(HarnessError::Invalid(format!(
                    "unsupported fixture entry: {}",
                    entry.path().display()
                )));
            }
        }
        Ok(())
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = self.directory.path();
    }
}

fn validate_relative_components(value: &str) -> Result<()> {
    let path = Path::new(value);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(HarnessError::Invalid(format!(
            "unsafe sandbox-relative path: {value}"
        )));
    }
    Ok(())
}

fn validate_environment_key(key: &str) -> Result<()> {
    if key.is_empty()
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        || key.starts_with("DYLD_")
        || key.starts_with("LD_")
        || key == "PATH"
    {
        return Err(HarnessError::Invalid(format!(
            "unsafe environment variable name: {key}"
        )));
    }
    Ok(())
}
