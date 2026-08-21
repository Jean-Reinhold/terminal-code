use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tode_core::{read_key, set_key};

pub const FONT_FAMILY: &str = "JetBrains Mono";
pub const FONT_FALLBACKS: &str = "Menlo, \"DejaVu Sans Mono\", \"Liberation Mono\", monospace";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilePaths {
    pub install_root: PathBuf,
    pub data: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
    pub runtime: PathBuf,
    pub logs: PathBuf,
    pub user: PathBuf,
    pub extensions: PathBuf,
    pub browser_data: PathBuf,
    pub browser_state: PathBuf,
    pub browser_cache: PathBuf,
}

impl ProfilePaths {
    pub fn from_environment(home: &Path, environment: &BTreeMap<OsString, OsString>) -> Self {
        let data_home = base(environment, "XDG_DATA_HOME", home, ".local/share");
        let state_home = base(environment, "XDG_STATE_HOME", home, ".local/state");
        let cache_home = base(environment, "XDG_CACHE_HOME", home, ".cache");
        let data = data_home.join("tode");
        let state = state_home.join("tode");
        let cache = cache_home.join("tode");
        let vscode = data.join("vscode");
        let install_root = environment
            .get(OsStr::new("TODE_INSTALL_ROOT"))
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".local/lib/tode"));
        Self {
            install_root,
            runtime: data.join("runtime"),
            logs: state.join("logs"),
            user: vscode.join("user-data/User"),
            extensions: vscode.join("extensions"),
            browser_data: data.join("browser/share"),
            browser_state: state.join("browser/state"),
            browser_cache: cache.join("browser"),
            data,
            state,
            cache,
        }
    }
}

pub fn apply_settings(source: &str) -> String {
    let mut text = source.to_owned();
    for (key, value) in seeded_settings() {
        if read_key(source, key).is_none() {
            text = set_key(&text, key, &value);
        }
    }
    for (key, value) in managed_settings() {
        text = set_key(&text, key, &value);
    }
    text
}

pub fn write_if_changed(path: &Path, contents: &[u8]) -> std::io::Result<bool> {
    if fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(false);
    }
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent")
    })?;
    fs::create_dir_all(parent)?;
    let existing_permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(contents)?;
    temporary.as_file().sync_all()?;
    if let Some(permissions) = existing_permissions {
        temporary.as_file().set_permissions(permissions)?;
    } else {
        temporary
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(true)
}

pub fn install_settings(paths: &ProfilePaths) -> std::io::Result<bool> {
    let file = paths.user.join("settings.json");
    let source = fs::read_to_string(&file).unwrap_or_else(|_| "{}".into());
    write_if_changed(&file, apply_settings(&source).as_bytes())
}

fn base(
    environment: &BTreeMap<OsString, OsString>,
    variable: &str,
    home: &Path,
    fallback: &str,
) -> PathBuf {
    environment
        .get(OsStr::new(variable))
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(fallback))
}

fn managed_settings() -> Vec<(&'static str, Value)> {
    let stack = format!("\"{FONT_FAMILY}\", {FONT_FALLBACKS}");
    vec![
        ("workbench.colorTheme", json!("Terminal Code")),
        ("editor.fontFamily", json!(stack)),
        ("terminal.integrated.fontFamily", json!(stack)),
        ("chat.editor.fontFamily", json!(stack)),
        ("debug.console.fontFamily", json!(stack)),
        ("markdown.preview.fontFamily", json!(stack)),
        ("terminal.integrated.enableImages", json!(true)),
        ("workbench.startupEditor", json!("none")),
        (
            "workbench.secondarySideBar.defaultVisibility",
            json!("hidden"),
        ),
        ("chat.commandCenter.enabled", json!(false)),
        ("workbench.tips.enabled", json!(false)),
        (
            "workbench.welcomePage.walkthroughs.openOnInstall",
            json!(false),
        ),
        ("window.commandCenter", json!(false)),
        ("window.title", json!("${dirty}${activeEditorShort}")),
        ("editor.smoothScrolling", json!(false)),
        ("workbench.list.smoothScrolling", json!(false)),
        ("terminal.integrated.smoothScrolling", json!(false)),
        ("update.mode", json!("none")),
        ("telemetry.telemetryLevel", json!("off")),
        ("workbench.enableExperiments", json!(false)),
    ]
}

fn seeded_settings() -> Vec<(&'static str, Value)> {
    vec![
        ("workbench.activityBar.location", json!("top")),
        ("editor.fontSize", json!(13)),
        ("workbench.tree.indent", json!(12)),
        ("editor.cursorBlinking", json!("solid")),
        ("editor.minimap.enabled", json!(false)),
        ("scm.defaultViewMode", json!("tree")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn paths_honor_only_absolute_overrides() {
        let home = Path::new("/home/test");
        let mut environment = BTreeMap::new();
        environment.insert("XDG_DATA_HOME".into(), "/data".into());
        environment.insert("XDG_STATE_HOME".into(), "relative-state".into());
        environment.insert("TODE_INSTALL_ROOT".into(), "/opt/tode".into());
        let paths = ProfilePaths::from_environment(home, &environment);
        assert_eq!(paths.data, Path::new("/data/tode"));
        assert_eq!(paths.state, Path::new("/home/test/.local/state/tode"));
        assert_eq!(paths.install_root, Path::new("/opt/tode"));
        assert_eq!(paths.user, Path::new("/data/tode/vscode/user-data/User"));
    }

    #[test]
    fn seeded_settings_never_override_user_values_and_managed_always_win() {
        let source = r#"{
  // user preference
  "editor.fontSize": 19,
  "workbench.colorTheme": "Monokai"
}
"#;
        let output = apply_settings(source);
        assert_eq!(read_key(&output, "editor.fontSize"), Some(json!(19)));
        assert_eq!(
            read_key(&output, "workbench.colorTheme"),
            Some(json!("Terminal Code"))
        );
        assert!(output.contains("// user preference"));
        assert_eq!(
            read_key(&output, "editor.minimap.enabled"),
            Some(json!(false))
        );
    }

    #[test]
    fn applying_settings_twice_is_byte_stable() {
        let once = apply_settings("{}");
        let twice = apply_settings(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn atomic_write_reports_changes_and_preserves_existing_mode() {
        let root = TempDir::new().unwrap();
        let file = root.path().join("settings.json");
        fs::write(&file, "old").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
        assert!(write_if_changed(&file, b"new").unwrap());
        assert_eq!(fs::read(&file).unwrap(), b"new");
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o640
        );
        assert!(!write_if_changed(&file, b"new").unwrap());
    }

    #[test]
    fn install_settings_creates_profile_and_is_idempotent() {
        let root = TempDir::new().unwrap();
        let paths = ProfilePaths::from_environment(root.path(), &BTreeMap::new());
        assert!(install_settings(&paths).unwrap());
        assert!(!install_settings(&paths).unwrap());
        let settings = fs::read_to_string(paths.user.join("settings.json")).unwrap();
        assert_eq!(
            read_key(&settings, "workbench.colorTheme"),
            Some(json!("Terminal Code"))
        );
    }
}
