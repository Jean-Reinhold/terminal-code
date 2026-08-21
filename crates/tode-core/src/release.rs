use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformBuild {
    pub file: String,
    pub sha256: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    pub channel: String,
    pub platforms: BTreeMap<String, PlatformBuild>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    pub version: String,
    pub channel: String,
    pub platform: String,
    pub file: String,
    pub sha256: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledReceipt {
    pub version: String,
    pub channel: String,
}

pub fn target_triple(platform: &str, architecture: &str) -> String {
    format!(
        "{}-{}",
        if platform == "darwin" {
            "darwin"
        } else {
            "linux"
        },
        if architecture == "arm64" {
            "arm64"
        } else {
            "x64"
        }
    )
}

pub fn current_target_triple() -> String {
    target_triple(
        if std::env::consts::OS == "macos" {
            "darwin"
        } else {
            "linux"
        },
        if std::env::consts::ARCH == "aarch64" {
            "arm64"
        } else {
            "x64"
        },
    )
}

pub fn build_for(manifest: &ReleaseManifest, target: &str) -> Result<Build, String> {
    let build = manifest
        .platforms
        .get(target)
        .ok_or_else(|| format!("tode {} has no build for {target}", manifest.version))?;
    Ok(Build {
        version: manifest.version.clone(),
        channel: manifest.channel.clone(),
        platform: target.into(),
        file: build.file.clone(),
        sha256: build.sha256.clone(),
        size: build.size,
        url: build.url.clone(),
    })
}

pub fn latest_manifest_path(channel: &str) -> String {
    if channel == "stable" {
        "/latest.json".into()
    } else {
        format!("/{channel}/latest.json")
    }
}

pub fn installed_receipt(root: &Path) -> Option<InstalledReceipt> {
    let version = read_trimmed(&root.join("VERSION"))?;
    let channel = read_trimmed(&root.join("CHANNEL"))?;
    Some(InstalledReceipt { version, channel })
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn target_mapping_matches_release_keys() {
        assert_eq!(target_triple("darwin", "arm64"), "darwin-arm64");
        assert_eq!(target_triple("darwin", "x86_64"), "darwin-x64");
        assert_eq!(target_triple("linux", "aarch64"), "linux-x64");
        assert_eq!(target_triple("win32", "arm64"), "linux-arm64");
    }

    #[test]
    fn selects_build_and_preserves_manifest_fields() {
        let mut platforms = BTreeMap::new();
        platforms.insert(
            "darwin-arm64".into(),
            PlatformBuild {
                file: "tode.tar.gz".into(),
                sha256: "abc".into(),
                size: 42,
                url: "https://example.test/tode.tar.gz".into(),
            },
        );
        let build = build_for(
            &ReleaseManifest {
                version: "v1.2.3".into(),
                channel: "stable".into(),
                platforms,
            },
            "darwin-arm64",
        )
        .unwrap();
        assert_eq!(build.version, "v1.2.3");
        assert_eq!(build.platform, "darwin-arm64");
        assert_eq!(build.size, 42);
    }

    #[test]
    fn missing_build_error_names_version_and_target() {
        let error = build_for(
            &ReleaseManifest {
                version: "v9".into(),
                channel: "dev".into(),
                platforms: BTreeMap::new(),
            },
            "linux-arm64",
        )
        .unwrap_err();
        assert_eq!(error, "tode v9 has no build for linux-arm64");
    }

    #[test]
    fn latest_paths_preserve_stable_and_named_channels() {
        assert_eq!(latest_manifest_path("stable"), "/latest.json");
        assert_eq!(latest_manifest_path("dev"), "/dev/latest.json");
    }

    #[test]
    fn installed_receipt_requires_both_nonempty_files() {
        let root = TempDir::new().unwrap();
        assert_eq!(installed_receipt(root.path()), None);
        fs::write(root.path().join("VERSION"), "v1\n").unwrap();
        assert_eq!(installed_receipt(root.path()), None);
        fs::write(root.path().join("CHANNEL"), "stable\n").unwrap();
        assert_eq!(
            installed_receipt(root.path()),
            Some(InstalledReceipt {
                version: "v1".into(),
                channel: "stable".into(),
            })
        );
    }
}
