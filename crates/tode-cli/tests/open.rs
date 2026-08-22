use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;

#[test]
fn opens_folder_through_rust_daemon_and_browser_then_shuts_down() {
    let root = TempDir::new().unwrap();
    let install = root.path().join("install");
    let runtime = install.join("vendor/terminal-browser");
    let electron = tode_runtime::electron_entry(&runtime, cfg!(target_os = "macos"));
    fs::create_dir_all(electron.parent().unwrap()).unwrap();
    fs::create_dir_all(runtime.join("cli/dist")).unwrap();
    fs::write(runtime.join("VERSION"), "v0.5.8\n").unwrap();
    fs::write(runtime.join("cli/dist/main.js"), "main").unwrap();
    fs::write(
        &electron,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$BROWSER_ARGS_FILE\"\n",
    )
    .unwrap();
    fs::set_permissions(&electron, fs::Permissions::from_mode(0o755)).unwrap();
    let workspace = root.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let browser_args = root.path().join("browser-args.txt");
    let target = workspace_target("tode");
    let daemon = workspace_target("tode-daemon");
    let code_server = workspace_target("tode-code-server-probe");
    assert!(
        daemon.is_file(),
        "build tode-runtime binaries before CLI integration"
    );
    assert!(
        code_server.is_file(),
        "build code-server probe before CLI integration"
    );

    let output = base_command(
        &target,
        root.path(),
        &install,
        &daemon,
        &code_server,
        &browser_args,
    )
    .current_dir(&workspace)
    .arg("--timing")
    .arg(".")
    .output()
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let arguments = fs::read_to_string(&browser_args).unwrap();
    assert!(
        arguments
            .lines()
            .next()
            .unwrap()
            .ends_with("cli/dist/main.js")
    );
    assert!(arguments.lines().any(|line| line == "open"));
    assert!(arguments.lines().any(|line| line == "--app-mode"));
    assert!(arguments.contains("folder=%2F"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("runtime"));
    assert!(stderr.contains("profile"));
    assert!(stderr.contains("code-server"));
    let preload = root.path().join("data/tode/browser-preload.js");
    let main_script = root.path().join("data/tode/browser-main.js");
    assert!(preload.is_file());
    assert!(main_script.is_file());
    assert!(
        arguments
            .lines()
            .any(|line| line == format!("--preload={}", preload.display()))
    );
    assert!(
        arguments
            .lines()
            .any(|line| line == format!("--main-script={}", main_script.display()))
    );
    let launch: serde_json::Value = serde_json::from_slice(
        &fs::read(root.path().join("data/tode/inject.css.launch.json")).unwrap(),
    )
    .unwrap();
    assert!(launch["spawnedAt"].as_i64().unwrap() > 0);
    assert_eq!(launch["stages"][0][0], "runtime");
    assert_eq!(launch["stages"][1][0], "profile");
    assert_eq!(launch["stages"][2][0], "code-server");

    let shutdown = base_command(
        &target,
        root.path(),
        &install,
        &daemon,
        &code_server,
        &browser_args,
    )
    .arg("--shutdown")
    .output()
    .unwrap();
    assert!(shutdown.status.success());
    assert_eq!(String::from_utf8_lossy(&shutdown.stdout), "tode stopped\n");
    let state = root.path().join("state/tode/server.json");
    let deadline = Instant::now() + Duration::from_secs(2);
    while state.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(!state.exists());
}

fn base_command(
    binary: &Path,
    root: &Path,
    install: &Path,
    daemon: &Path,
    code_server: &Path,
    browser_args: &Path,
) -> Command {
    let mut command = Command::new(binary);
    command
        .env("HOME", root.join("home"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("TODE_INSTALL_ROOT", install)
        .env("TODE_DAEMON", daemon)
        .env("TODE_CODE_SERVER", code_server)
        .env("BROWSER_ARGS_FILE", browser_args);
    command
}

fn workspace_target(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug")
        .join(name)
}
