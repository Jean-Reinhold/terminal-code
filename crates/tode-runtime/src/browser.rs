use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

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
    use tempfile::TempDir;

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
}
