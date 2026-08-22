use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::TempDir;

#[test]
fn reports_missing_and_recorded_page_timing() {
    let root = TempDir::new().unwrap();
    let binary = env!("CARGO_BIN_EXE_tode");
    let missing = base(binary, root.path()).arg("--timing").output().unwrap();
    assert!(missing.status.success());
    assert_eq!(
        String::from_utf8_lossy(&missing.stdout),
        "no page timing recorded yet, open tode once\n"
    );

    let data = root.path().join("data/tode");
    std::fs::create_dir_all(&data).unwrap();
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    std::fs::write(
        data.join("inject.css.timing.json"),
        serde_json::to_vec(&serde_json::json!({
            "at": at,
            "origin": at + 50,
            "responseEnd": 20,
            "loadEnd": 200,
            "domInteractive": 80,
            "marks": {
                "code/didStartRenderer": 100,
                "code/didStartWorkbench": 180
            }
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        data.join("inject.css.launch.json"),
        serde_json::to_vec(&serde_json::json!({
            "spawnedAt": at,
            "stages": [["runtime", 10], ["profile", 20]]
        }))
        .unwrap(),
    )
    .unwrap();
    let output = base(binary, root.path()).arg("--timing").output().unwrap();
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).unwrap();
    assert!(output.contains("page load, 0s ago"));
    assert!(output.contains("tode: runtime"));
    assert!(output.contains("document arrived"));
    assert!(output.contains("renderer started"));
    assert!(output.contains('█'));
}

fn base(binary: &str, root: &std::path::Path) -> Command {
    let mut command = Command::new(binary);
    command
        .env("HOME", root.join("home"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("XDG_CACHE_HOME", root.join("cache"));
    command
}
