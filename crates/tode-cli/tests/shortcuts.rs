use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn reports_unsupported_terminal_without_error() {
    let root = TempDir::new().unwrap();
    let output = base(&root)
        .env("TERM", "dumb")
        .arg("--shortcut-setup")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "shortcut setup not yet available in this terminal, please file an issue if you want your terminal supported https://github.com/zenbu-labs/terminal-code/issues"
    );
}

#[test]
fn detected_terminal_requires_cli_for_scanning() {
    let root = TempDir::new().unwrap();
    let output = ghostty(base(&root), &root)
        .arg("--shortcut-setup")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "the ghostty cli is not on PATH, so its keybinds cannot be read\n"
    );
}

#[test]
fn undo_works_without_cli_and_preserves_user_config() {
    let root = TempDir::new().unwrap();
    let config = root.path().join("config/ghostty");
    fs::create_dir_all(config.join("tode")).unwrap();
    fs::write(
        config.join("config"),
        "font-size = 14\nconfig-file = ?tode/keybinds.ghostty\n",
    )
    .unwrap();
    fs::write(config.join("tode/keybinds.ghostty"), "owned\n").unwrap();
    let decisions = root.path().join("data/tode/shortcuts.json");
    fs::create_dir_all(decisions.parent().unwrap()).unwrap();
    fs::write(
        &decisions,
        "{\"version\":1,\"terminal\":\"ghostty\",\"choices\":{}}\n",
    )
    .unwrap();
    let output = ghostty(base(&root), &root)
        .args(["--shortcut-setup", "--undo", "--no-boot"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .starts_with("removed tode's Ghostty overrides and editor chords\nreload ghostty")
    );
    assert_eq!(
        fs::read_to_string(config.join("config")).unwrap(),
        "font-size = 14\n"
    );
    assert!(!config.join("tode/keybinds.ghostty").exists());
    assert!(!decisions.exists());
}

#[test]
fn harmless_effective_binding_reports_no_conflicts() {
    let root = TempDir::new().unwrap();
    install_ghostty(&root, "printf 'keybind = ctrl+c=copy_to_clipboard\\n'");
    let output = ghostty(base(&root), &root)
        .arg("--shortcut-setup")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "no shortcut conflicts detected!\n"
    );
}

#[test]
fn non_tty_conflict_directs_user_to_terminal() {
    let root = TempDir::new().unwrap();
    install_ghostty(&root, "printf 'keybind = ctrl+p=new_tab\\n'");
    let output = ghostty(base(&root), &root)
        .arg("--shortcut-setup")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "run tode --shortcut-setup in a terminal to continue\n"
    );
}

fn base(root: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tode"));
    command
        .env("HOME", root.path().join("home"))
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env("XDG_CACHE_HOME", root.path().join("cache"))
        .env("XDG_CONFIG_HOME", root.path().join("config"))
        .env("PATH", root.path().join("bin"))
        .env_remove("TERM_PROGRAM")
        .env_remove("GHOSTTY_RESOURCES_DIR")
        .env_remove("KITTY_WINDOW_ID")
        .env_remove("KITTY_PID");
    command
}

fn ghostty(mut command: Command, root: &TempDir) -> Command {
    command
        .env("TERM_PROGRAM", "ghostty")
        .env("PATH", root.path().join("bin"));
    command
}

fn install_ghostty(root: &TempDir, body: &str) {
    let binary = root.path().join("bin/ghostty");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut permissions = fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(binary, permissions).unwrap();
}
