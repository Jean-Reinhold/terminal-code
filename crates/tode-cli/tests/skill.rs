use std::fs;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn prints_resolved_read_only_install_and_profile_state() {
    let root = TempDir::new().unwrap();
    let home = root.path().join("home");
    let install = root.path().join("install");
    let data = root.path().join("data");
    let config = root.path().join("config");
    fs::create_dir_all(install.join("vendor/terminal-browser")).unwrap();
    fs::write(install.join("VERSION"), "v9\n").unwrap();
    fs::write(install.join("CHANNEL"), "stable\n").unwrap();
    fs::create_dir_all(data.join("tode/vscode/extensions/acme.extension")).unwrap();
    fs::create_dir_all(data.join("tode/vscode/extensions/.hidden")).unwrap();
    fs::create_dir_all(config.join("ghostty")).unwrap();
    fs::write(config.join("ghostty/config"), "font-size = 14\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tode"))
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &data)
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env("XDG_CACHE_HOME", root.path().join("cache"))
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_BIN_HOME", root.path().join("bin"))
        .env("TODE_INSTALL_ROOT", &install)
        .env("TERM_PROGRAM", "ghostty")
        .env_remove("TODE_IPC")
        .env_remove("TODE_TERMINAL_BROWSER_BIN")
        .args(["--skill", "ignored-by-legacy-dispatch"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let skill = String::from_utf8(output.stdout).unwrap();
    assert!(skill.starts_with("---\nname: tode\n"));
    assert!(skill.contains(&format!(
        "install root: {} — release v9 on channel stable",
        install.display()
    )));
    assert!(skill.contains(&format!(
        "vendored in {}",
        install.join("vendor/terminal-browser").display()
    )));
    assert!(skill.contains("daemon: not running"));
    assert!(skill.contains("open windows: 0 socket(s)"));
    assert!(skill.contains(&format!(
        "ghostty, config in {}",
        config.join("ghostty").display()
    )));
    assert!(skill.contains("installed extensions (1): acme.extension"));
    assert!(skill.contains(&data.join("tode/live-theme.json").display().to_string()));
    assert!(skill.contains("workbench.colorTheme"));
    assert!(skill.ends_with("--shutdown and --uninstall.\n"));
}
