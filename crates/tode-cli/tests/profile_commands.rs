use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn imports_discovered_editor_and_installs_theme() {
    let root = TempDir::new().unwrap();
    let home = root.path().join("home");
    let user = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Code/User")
    } else {
        home.join(".config/Code/User")
    };
    fs::create_dir_all(user.join("globalStorage")).unwrap();
    fs::write(user.join("globalStorage/state.vscdb"), "state").unwrap();
    fs::write(
        user.join("settings.json"),
        "{\"editor.fontSize\":19,\"workbench.colorTheme\":\"Old\"}",
    )
    .unwrap();
    let source_extensions = home.join(".vscode/extensions");
    fs::create_dir_all(&source_extensions).unwrap();
    fs::write(source_extensions.join("extensions.json"), "[]").unwrap();

    let binary = env!("CARGO_BIN_EXE_tode");
    let imported = base(binary, root.path(), &home)
        .args(["--import", "Code"])
        .output()
        .unwrap();
    assert!(
        imported.status.success(),
        "{}",
        String::from_utf8_lossy(&imported.stderr)
    );
    let report: Value = serde_json::from_slice(&imported.stdout).unwrap();
    assert_eq!(report["settings"]["imported"], 2);
    assert_eq!(
        report["settings"]["kept_by_tode"][0],
        "workbench.colorTheme"
    );
    let profile = root
        .path()
        .join("data/tode/vscode/user-data/User/settings.json");
    let settings = fs::read_to_string(profile).unwrap();
    assert_eq!(
        tode_core::read_key(&settings, "workbench.colorTheme"),
        Some(serde_json::json!("Terminal Code"))
    );

    let themed = base(binary, root.path(), &home)
        .arg("--theme")
        .output()
        .unwrap();
    assert!(
        themed.status.success(),
        "{}",
        String::from_utf8_lossy(&themed.stderr)
    );
    assert!(String::from_utf8_lossy(&themed.stdout).starts_with("theme "));
    let extensions = root.path().join("data/tode/vscode/extensions");
    assert!(fs::read_dir(&extensions).unwrap().flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .starts_with("tode.tode-theme-")
    }));
    assert!(root.path().join("data/tode/live-theme.json").is_file());
    let custom = root.path().join("custom-theme.json");
    fs::write(
        &custom,
        r##"{
          // JSONC is accepted like VS Code theme files
          "name": "Custom",
          "type": "light",
          "colors": {"editor.background": "#abcdef"},
          "tokenColors": [],
        }"##,
    )
    .unwrap();
    let custom_output = base(binary, root.path(), &home)
        .args(["--theme", custom.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        custom_output.status.success(),
        "{}",
        String::from_utf8_lossy(&custom_output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&custom_output.stdout),
        format!(
            "theme set from {} — open windows follow without a reload\n",
            custom.display()
        )
    );
    let live: Value =
        serde_json::from_slice(&fs::read(root.path().join("data/tode/live-theme.json")).unwrap())
            .unwrap();
    assert_eq!(live["colors"]["editor.background"], "#abcdef");
    let invalid = root.path().join("invalid-theme.json");
    fs::write(&invalid, "{}").unwrap();
    let invalid_output = base(binary, root.path(), &home)
        .args(["--theme", invalid.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(invalid_output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&invalid_output.stderr),
        format!(
            "tode: {} is not a vscode theme (expected a json document with colors or tokenColors)\n",
            invalid.display()
        )
    );
}

fn base(binary: &str, root: &std::path::Path, home: &std::path::Path) -> Command {
    let mut command = Command::new(binary);
    command
        .env("HOME", home)
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("XDG_CACHE_HOME", root.join("cache"));
    command
}
