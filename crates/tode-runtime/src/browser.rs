use reqwest::Client;
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::{download_verified, unpack_tar_gz_stripped};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSource {
    Override,
    Vendored,
    Pinned,
    Cloned,
    Downloaded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserRuntime {
    pub bin: PathBuf,
    pub root: PathBuf,
    pub version: String,
    pub source: RuntimeSource,
}

#[derive(Debug, Clone)]
pub struct BrowserHomes {
    pub data: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
    pub app_data: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RuntimeRoots {
    pub runtime: PathBuf,
    pub vendored: PathBuf,
    pub system_install: PathBuf,
    pub homes: BrowserHomes,
}

#[derive(Debug, Clone, Deserialize)]
struct BrowserRelease {
    version: String,
    url: String,
    sha256: String,
    size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserResolveError {
    #[error("{0}")]
    Existing(String),
    #[error("read terminal-browser release: {0}")]
    Release(#[from] reqwest::Error),
    #[error(transparent)]
    Artifact(#[from] crate::ArtifactError),
    #[error("resolved terminal-browser is missing pieces: {0}")]
    MissingPieces(String),
    #[error("write terminal-browser launcher: {0}")]
    Launcher(std::io::Error),
}

pub fn electron_entry(root: &Path, is_macos: bool) -> PathBuf {
    if is_macos {
        root.join("electron/terminal-browser.app/Contents/MacOS/terminal-browser")
    } else {
        root.join("electron/electron")
    }
}

pub fn version_at(root: &Path) -> Option<String> {
    fs::read_to_string(root.join("VERSION"))
        .ok()
        .map(|version| version.trim().to_owned())
        .filter(|version| !version.is_empty())
}

pub fn usable(root: &Path, version: &str, is_macos: bool) -> bool {
    version_at(root).as_deref() == Some(version)
        && root.join("cli/dist/main.js").is_file()
        && electron_entry(root, is_macos).is_file()
}

pub fn resolve_existing(
    roots: &RuntimeRoots,
    version: &str,
    pinned_version: &str,
    override_bin: Option<&Path>,
    is_macos: bool,
) -> Result<Option<BrowserRuntime>, String> {
    if let Some(binary) = override_bin {
        if !binary.is_file() {
            return Err(format!(
                "TODE_TERMINAL_BROWSER_BIN is not there: {}",
                binary.display()
            ));
        }
        let root = binary
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| "terminal-browser override has no root".to_owned())?
            .to_path_buf();
        return Ok(Some(BrowserRuntime {
            bin: binary.to_path_buf(),
            version: version_at(&root).unwrap_or_else(|| "override".into()),
            root,
            source: RuntimeSource::Override,
        }));
    }
    if version == pinned_version && usable(&roots.vendored, version, is_macos) {
        return Ok(Some(runtime_with_launcher(
            roots,
            roots.vendored.clone(),
            version,
            RuntimeSource::Vendored,
            is_macos,
        )?));
    }
    let root = roots.runtime.join("terminal-browser").join(version);
    if usable(&root, version, is_macos) {
        return Ok(Some(runtime_with_launcher(
            roots,
            root,
            version,
            RuntimeSource::Pinned,
            is_macos,
        )?));
    }
    if usable(&roots.system_install, version, is_macos) {
        clone_tree(&roots.system_install, &root).map_err(|error| error.to_string())?;
        return Ok(Some(runtime_with_launcher(
            roots,
            root,
            version,
            RuntimeSource::Cloned,
            is_macos,
        )?));
    }
    Ok(None)
}

pub async fn resolve_runtime(
    client: &Client,
    roots: &RuntimeRoots,
    version: &str,
    pinned_version: &str,
    override_bin: Option<&Path>,
    is_macos: bool,
    release_origin: &str,
) -> Result<BrowserRuntime, BrowserResolveError> {
    if let Some(runtime) = resolve_existing(roots, version, pinned_version, override_bin, is_macos)
        .map_err(BrowserResolveError::Existing)?
    {
        return Ok(runtime);
    }
    let release: BrowserRelease = client
        .get(format!(
            "{}/v/{version}",
            release_origin.trim_end_matches('/')
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if release.version != version {
        return Err(BrowserResolveError::MissingPieces(format!(
            "release returned version {} for {version}",
            release.version
        )));
    }
    let tarball = roots.runtime.join(format!("{version}.tar.gz"));
    download_verified(
        client,
        &release.url,
        &release.sha256,
        release.size,
        &tarball,
    )
    .await?;
    let root = roots.runtime.join("terminal-browser").join(version);
    let unpacked = unpack_tar_gz_stripped(&tarball, &root, 1, 200_000, 2 * 1024 * 1024 * 1024);
    let _ = fs::remove_file(&tarball);
    unpacked?;
    if !usable(&root, version, is_macos) {
        return Err(BrowserResolveError::MissingPieces(version.into()));
    }
    let bin =
        write_launcher(&root, &roots.homes, is_macos).map_err(BrowserResolveError::Launcher)?;
    Ok(BrowserRuntime {
        bin,
        root,
        version: version.into(),
        source: RuntimeSource::Downloaded,
    })
}

pub fn write_launcher(
    root: &Path,
    homes: &BrowserHomes,
    is_macos: bool,
) -> std::io::Result<PathBuf> {
    let bin = root.join("bin/terminal-browser");
    fs::create_dir_all(bin.parent().expect("launcher has parent"))?;
    for directory in [&homes.data, &homes.state, &homes.cache, &homes.app_data] {
        fs::create_dir_all(directory)?;
    }
    let electron_entry = electron_entry(root, is_macos);
    let electron = electron_entry
        .strip_prefix(root)
        .expect("electron entry stays under runtime")
        .to_string_lossy();
    let scroll_helper = if is_macos {
        "export NATIVE_SCROLL_HELPER=\"${NATIVE_SCROLL_HELPER:-$ROOT/bin/native-scroll-helper}\"\n"
    } else {
        ""
    };
    let script = format!(
        "#!/bin/sh\nROOT=\"$(CDPATH= cd -- \"$(dirname -- \"$0\")/..\" && pwd -P)\"\nexport TERMINAL_BROWSER_DIST_ROOT=\"$ROOT\"\nexport ELECTRON_RUN_AS_NODE=1\n{scroll_helper}export XDG_DATA_HOME=${{TODE_BROWSER_DATA:-{}}}\nexport XDG_STATE_HOME=${{TODE_BROWSER_STATE:-{}}}\nexport XDG_CACHE_HOME=${{TODE_BROWSER_CACHE:-{}}}\nif [ -n \"${{TODE_BROWSER_RUN:-}}\" ]; then export XDG_RUNTIME_DIR=\"$TODE_BROWSER_RUN\"; fi\nexport TERMINAL_BROWSER_APPDATA=${{TODE_BROWSER_APPDATA:-{}}}\nexec \"$ROOT/{electron}\" \"$ROOT/cli/dist/main.js\" \"$@\"\n",
        shell_quote(&homes.data),
        shell_quote(&homes.state),
        shell_quote(&homes.cache),
        shell_quote(&homes.app_data),
    );
    let mut file = fs::File::create(&bin)?;
    file.write_all(script.as_bytes())?;
    file.sync_all()?;
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755))?;
    Ok(bin)
}

fn runtime_with_launcher(
    roots: &RuntimeRoots,
    root: PathBuf,
    version: &str,
    source: RuntimeSource,
    is_macos: bool,
) -> Result<BrowserRuntime, String> {
    let bin = write_launcher(&root, &roots.homes, is_macos).map_err(|error| error.to_string())?;
    Ok(BrowserRuntime {
        bin,
        root,
        version: version.into(),
        source,
    })
}

fn clone_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    let staging = target.with_extension("cloning");
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .expect("walk stays under source");
        let destination = staging.join(relative);
        if entry.file_type().is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "runtime tree contains symlink",
            ));
        }
        if entry.file_type().is_dir() {
            fs::create_dir_all(destination)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), destination)?;
        }
    }
    if target.exists() {
        fs::remove_dir_all(target)?;
    }
    fs::rename(staging, target)
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    fn roots(root: &TempDir) -> RuntimeRoots {
        RuntimeRoots {
            runtime: root.path().join("data/runtime"),
            vendored: root.path().join("install/vendor/terminal-browser"),
            system_install: root.path().join("system"),
            homes: BrowserHomes {
                data: root.path().join("browser/data"),
                state: root.path().join("browser/state"),
                cache: root.path().join("browser/cache"),
                app_data: root.path().join("browser/appdata"),
            },
        }
    }

    fn make_runtime(root: &Path, version: &str, is_macos: bool) {
        fs::create_dir_all(root.join("cli/dist")).unwrap();
        fs::write(root.join("cli/dist/main.js"), "main").unwrap();
        let electron = electron_entry(root, is_macos);
        fs::create_dir_all(electron.parent().unwrap()).unwrap();
        fs::write(electron, "electron").unwrap();
        fs::write(root.join("VERSION"), format!("{version}\n")).unwrap();
    }

    #[test]
    fn platform_entry_and_usable_require_every_piece() {
        let root = TempDir::new().unwrap();
        assert!(
            electron_entry(root.path(), true)
                .ends_with("terminal-browser.app/Contents/MacOS/terminal-browser")
        );
        assert!(electron_entry(root.path(), false).ends_with("electron/electron"));
        make_runtime(root.path(), "v1", false);
        assert!(usable(root.path(), "v1", false));
        assert!(!usable(root.path(), "v2", false));
    }

    #[test]
    fn override_vendored_and_pinned_precedence_is_stable() {
        let root = TempDir::new().unwrap();
        let roots = roots(&root);
        let override_root = root.path().join("override");
        fs::create_dir_all(override_root.join("bin")).unwrap();
        let override_bin = override_root.join("bin/terminal-browser");
        fs::write(&override_bin, "bin").unwrap();
        assert_eq!(
            resolve_existing(&roots, "v1", "v1", Some(&override_bin), false)
                .unwrap()
                .unwrap()
                .source,
            RuntimeSource::Override
        );
        make_runtime(&roots.vendored, "v1", false);
        assert_eq!(
            resolve_existing(&roots, "v1", "v1", None, false)
                .unwrap()
                .unwrap()
                .source,
            RuntimeSource::Vendored
        );
        make_runtime(&roots.runtime.join("terminal-browser/v2"), "v2", false);
        assert_eq!(
            resolve_existing(&roots, "v2", "v1", None, false)
                .unwrap()
                .unwrap()
                .source,
            RuntimeSource::Pinned
        );
    }

    #[test]
    fn clones_existing_system_install_when_needed() {
        let root = TempDir::new().unwrap();
        let roots = roots(&root);
        make_runtime(&roots.system_install, "v3", false);
        let runtime = resolve_existing(&roots, "v3", "v1", None, false)
            .unwrap()
            .unwrap();
        assert_eq!(runtime.source, RuntimeSource::Cloned);
        assert!(runtime.root.join("cli/dist/main.js").is_file());
    }

    #[test]
    fn launcher_exports_browser_homes_and_is_executable() {
        let root = TempDir::new().unwrap();
        let roots = roots(&root);
        make_runtime(&roots.vendored, "v1", true);
        let launcher = write_launcher(&roots.vendored, &roots.homes, true).unwrap();
        let source = fs::read_to_string(&launcher).unwrap();
        assert!(source.contains("TERMINAL_BROWSER_DIST_ROOT"));
        assert!(source.contains("NATIVE_SCROLL_HELPER"));
        assert!(source.contains("TODE_BROWSER_DATA"));
        assert!(source.contains("cli/dist/main.js"));
        assert_eq!(
            fs::metadata(launcher).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }

    #[tokio::test]
    async fn downloads_verifies_unpacks_and_launches_missing_runtime() {
        let root = TempDir::new().unwrap();
        let roots = roots(&root);
        let archive = runtime_archive("v4");
        let digest = hex::encode(Sha256::digest(&archive));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let release = serde_json::to_vec(&serde_json::json!({
            "version": "v4",
            "channel": "stable",
            "url": format!("http://{address}/artifact"),
            "sha256": digest,
            "size": archive.len()
        }))
        .unwrap();
        let server = tokio::spawn(async move {
            for body in [release, archive] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = [0_u8; 2048];
                let _ = stream.read(&mut request).await.unwrap();
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
                stream.write_all(&body).await.unwrap();
            }
        });
        let runtime = resolve_runtime(
            &Client::new(),
            &roots,
            "v4",
            "v1",
            None,
            false,
            &format!("http://{address}"),
        )
        .await
        .unwrap();
        server.await.unwrap();
        assert_eq!(runtime.source, RuntimeSource::Downloaded);
        assert!(usable(&runtime.root, "v4", false));
        assert!(runtime.bin.is_file());
        assert!(!roots.runtime.join("v4.tar.gz").exists());
    }

    fn runtime_archive(version: &str) -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut archive = tar::Builder::new(encoder);
        for (path, contents, mode) in [
            ("bundle/VERSION", version.as_bytes(), 0o644),
            ("bundle/cli/dist/main.js", b"main".as_slice(), 0o644),
            ("bundle/electron/electron", b"electron".as_slice(), 0o755),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(mode);
            header.set_cksum();
            archive.append_data(&mut header, path, contents).unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap()
    }
}
