use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tode_core::{ghostty_without_include, kitty_without_include};

use crate::ProfilePaths;

#[derive(Debug, Clone)]
pub struct UninstallConfig {
    pub paths: ProfilePaths,
    pub install_roots: Vec<PathBuf>,
    pub shim: PathBuf,
    pub font: PathBuf,
    pub bundled_font: PathBuf,
    pub ghostty_config: PathBuf,
    pub kitty_config: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UninstallReport {
    pub data: bool,
    pub state: bool,
    pub cache: bool,
    pub installs: usize,
    pub shim: bool,
    pub font: bool,
    pub ghostty: bool,
    pub kitty: bool,
}

pub fn uninstall(config: &UninstallConfig) -> std::io::Result<UninstallReport> {
    let ghostty = remove_terminal_config(
        &config.ghostty_config,
        "config",
        "tode/keybinds.ghostty",
        ghostty_without_include,
    )?;
    let kitty = remove_terminal_config(
        &config.kitty_config,
        "kitty.conf",
        "tode/keybinds.kitty.conf",
        kitty_without_include,
    )?;
    let font = remove_matching_file(&config.font, &config.bundled_font)?;
    let data = remove_dir(&config.paths.data)?;
    let state = remove_dir(&config.paths.state)?;
    let cache = remove_dir(&config.paths.cache)?;
    let mut installs = 0;
    let mut roots = config.install_roots.clone();
    roots.sort();
    roots.dedup();
    for root in roots {
        if root.join("VERSION").is_file() && remove_dir(&root)? {
            installs += 1;
        }
    }
    let shim = remove_owned_shim(&config.shim)?;
    Ok(UninstallReport {
        data,
        state,
        cache,
        installs,
        shim,
        font,
        ghostty,
        kitty,
    })
}

fn remove_terminal_config(
    root: &Path,
    config_name: &str,
    owned_name: &str,
    strip: fn(&str) -> String,
) -> std::io::Result<bool> {
    let owned = root.join(owned_name);
    let config = root.join(config_name);
    let mut changed = false;
    if owned.exists() {
        fs::remove_file(&owned)?;
        changed = true;
    }
    if let Ok(source) = fs::read_to_string(&config) {
        let stripped = strip(&source);
        if stripped != source {
            fs::write(config, stripped)?;
            changed = true;
        }
    }
    Ok(changed)
}

fn remove_matching_file(target: &Path, ours: &Path) -> std::io::Result<bool> {
    let (Ok(target_bytes), Ok(our_bytes)) = (fs::read(target), fs::read(ours)) else {
        return Ok(false);
    };
    if target_bytes != our_bytes {
        return Ok(false);
    }
    fs::remove_file(target)?;
    Ok(true)
}

fn remove_owned_shim(path: &Path) -> std::io::Result<bool> {
    let Ok(source) = fs::read_to_string(path) else {
        return Ok(false);
    };
    if !source.contains("TODE_INSTALL_ROOT") {
        return Ok(false);
    }
    fs::remove_file(path)?;
    Ok(true)
}

fn remove_dir(path: &Path) -> std::io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(path)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn removes_only_owned_state_and_preserves_unrelated_terminal_config() {
        let root = TempDir::new().unwrap();
        let paths = ProfilePaths::from_environment(root.path(), &BTreeMap::new());
        for directory in [&paths.data, &paths.state, &paths.cache] {
            fs::create_dir_all(directory).unwrap();
            fs::write(directory.join("owned"), "owned").unwrap();
        }
        let install = root.path().join("install");
        fs::create_dir(&install).unwrap();
        fs::write(install.join("VERSION"), "v1").unwrap();
        let ghostty = root.path().join("ghostty");
        fs::create_dir_all(ghostty.join("tode")).unwrap();
        fs::write(
            ghostty.join("config"),
            "font-size = 14\nconfig-file = ?tode/keybinds.ghostty\n",
        )
        .unwrap();
        fs::write(ghostty.join("tode/keybinds.ghostty"), "owned").unwrap();
        let kitty = root.path().join("kitty");
        fs::create_dir_all(kitty.join("tode")).unwrap();
        fs::write(
            kitty.join("kitty.conf"),
            "font_size 14\ninclude tode/keybinds.kitty.conf\n",
        )
        .unwrap();
        fs::write(kitty.join("tode/keybinds.kitty.conf"), "owned").unwrap();
        let bundled_font = root.path().join("bundled.ttf");
        let font = root.path().join("font.ttf");
        fs::write(&bundled_font, "font").unwrap();
        fs::write(&font, "font").unwrap();
        let shim = root.path().join("tode");
        fs::write(&shim, "ROOT=${TODE_INSTALL_ROOT:-x}").unwrap();
        let report = uninstall(&UninstallConfig {
            paths: paths.clone(),
            install_roots: vec![install.clone()],
            shim: shim.clone(),
            font: font.clone(),
            bundled_font,
            ghostty_config: ghostty.clone(),
            kitty_config: kitty.clone(),
        })
        .unwrap();
        assert_eq!(report.installs, 1);
        assert!(report.data && report.state && report.cache);
        assert!(report.shim && report.font && report.ghostty && report.kitty);
        assert!(!install.exists());
        assert!(!shim.exists());
        assert!(!font.exists());
        assert_eq!(
            fs::read_to_string(ghostty.join("config")).unwrap(),
            "font-size = 14\n"
        );
        assert_eq!(
            fs::read_to_string(kitty.join("kitty.conf")).unwrap(),
            "font_size 14\n"
        );
    }

    #[test]
    fn refuses_foreign_font_shim_and_install_without_receipt() {
        let root = TempDir::new().unwrap();
        let paths = ProfilePaths::from_environment(root.path(), &BTreeMap::new());
        let install = root.path().join("install");
        fs::create_dir(&install).unwrap();
        let font = root.path().join("font.ttf");
        let bundled_font = root.path().join("bundled.ttf");
        fs::write(&font, "theirs").unwrap();
        fs::write(&bundled_font, "ours").unwrap();
        let shim = root.path().join("tode");
        fs::write(&shim, "#!/bin/sh\necho foreign\n").unwrap();
        let report = uninstall(&UninstallConfig {
            paths,
            install_roots: vec![install.clone()],
            shim: shim.clone(),
            font: font.clone(),
            bundled_font,
            ghostty_config: root.path().join("ghostty"),
            kitty_config: root.path().join("kitty"),
        })
        .unwrap();
        assert_eq!(report.installs, 0);
        assert!(!report.shim && !report.font);
        assert!(install.exists() && shim.exists() && font.exists());
    }

    #[test]
    fn repeated_uninstall_is_idempotent() {
        let root = TempDir::new().unwrap();
        let config = UninstallConfig {
            paths: ProfilePaths::from_environment(root.path(), &BTreeMap::new()),
            install_roots: Vec::new(),
            shim: root.path().join("shim"),
            font: root.path().join("font"),
            bundled_font: root.path().join("bundled"),
            ghostty_config: root.path().join("ghostty"),
            kitty_config: root.path().join("kitty"),
        };
        assert_eq!(uninstall(&config).unwrap(), UninstallReport::default());
        assert_eq!(uninstall(&config).unwrap(), UninstallReport::default());
    }
}
