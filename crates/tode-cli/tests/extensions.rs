use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn uninstalls_before_installs_and_lists_with_managed_paths() {
    let root = TempDir::new().unwrap();
    let script = root.path().join("code-server");
    let calls = root.path().join("calls.txt");
    fs::write(
        &script,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$CALLS\"\nif [ \"$1\" = \"--list-extensions\" ]; then echo 'example.one@1.0.0'; fi\n",
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let binary = env!("CARGO_BIN_EXE_tode");
    let output = base(binary, root.path(), &script, &calls)
        .args([
            "--install-extension",
            "new.one",
            "--uninstall-extension",
            "old.one",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "open tode again to pick it up\n"
    );
    let logged = fs::read_to_string(&calls).unwrap();
    let lines: Vec<_> = logged.lines().collect();
    assert!(lines[0].starts_with("--uninstall-extension old.one"));
    assert!(lines[1].starts_with("--install-extension new.one"));
    for line in &lines {
        assert!(line.contains("--extensions-dir"));
        assert!(line.contains("--user-data-dir"));
    }

    let listed = base(binary, root.path(), &script, &calls)
        .args(["--list-extensions", "--show-versions"])
        .output()
        .unwrap();
    assert!(listed.status.success());
    assert_eq!(
        String::from_utf8_lossy(&listed.stdout),
        "example.one@1.0.0\n"
    );
    assert!(
        fs::read_to_string(&calls)
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .starts_with("--list-extensions --show-versions")
    );
}

fn base(
    binary: &str,
    root: &std::path::Path,
    script: &std::path::Path,
    calls: &std::path::Path,
) -> Command {
    let mut command = Command::new(binary);
    command
        .env("HOME", root.join("home"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("TODE_CODE_SERVER", script)
        .env("CALLS", calls);
    command
}
