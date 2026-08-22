use std::fs;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn removes_owned_install_profile_shim_font_and_terminal_includes() {
    let root = TempDir::new().unwrap();
    let home = root.path().join("home");
    let install = root.path().join("install");
    let data = root.path().join("data/tode");
    let state = root.path().join("state/tode");
    let cache = root.path().join("cache/tode");
    for directory in [&install, &data, &state, &cache] {
        fs::create_dir_all(directory).unwrap();
    }
    fs::write(install.join("VERSION"), "v1").unwrap();
    let bundled_font = install.join("assets/fonts/JetBrainsMono-Regular.ttf");
    fs::create_dir_all(bundled_font.parent().unwrap()).unwrap();
    fs::write(&bundled_font, "font").unwrap();
    let font = if cfg!(target_os = "macos") {
        home.join("Library/Fonts/JetBrainsMono-Regular.ttf")
    } else {
        root.path().join("data/fonts/JetBrainsMono-Regular.ttf")
    };
    fs::create_dir_all(font.parent().unwrap()).unwrap();
    fs::write(&font, "font").unwrap();
    let bin = root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let shim = bin.join("tode");
    fs::write(&shim, "ROOT=${TODE_INSTALL_ROOT:-x}").unwrap();
    let config = root.path().join("config");
    fs::create_dir_all(config.join("ghostty/tode")).unwrap();
    fs::create_dir_all(config.join("kitty/tode")).unwrap();
    fs::write(
        config.join("ghostty/config"),
        "font-size = 14\nconfig-file = ?tode/keybinds.ghostty\n",
    )
    .unwrap();
    fs::write(config.join("ghostty/tode/keybinds.ghostty"), "owned").unwrap();
    fs::write(
        config.join("kitty/kitty.conf"),
        "font_size 14\ninclude tode/keybinds.kitty.conf\n",
    )
    .unwrap();
    fs::write(config.join("kitty/tode/keybinds.kitty.conf"), "owned").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tode"))
        .env("HOME", &home)
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env("XDG_CACHE_HOME", root.path().join("cache"))
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_BIN_HOME", &bin)
        .env("TODE_INSTALL_ROOT", &install)
        .args(["--uninstall", "--yes"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "done\n");
    assert!(!install.exists() && !data.exists() && !state.exists() && !cache.exists());
    assert!(!shim.exists() && !font.exists());
    assert_eq!(
        fs::read_to_string(config.join("ghostty/config")).unwrap(),
        "font-size = 14\n"
    );
    assert_eq!(
        fs::read_to_string(config.join("kitty/kitty.conf")).unwrap(),
        "font_size 14\n"
    );
}
