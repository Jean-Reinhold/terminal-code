use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn generates_theme_and_rejects_missing_required_colors() {
    let valid = run(r#"{"background":[1,2,3],"foreground":[240,241,242],"ansi":[]}"#);
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let theme: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(theme["colors"]["editor.background"], "#010203");
    assert!(theme["tokenColors"].as_array().unwrap().len() > 10);

    let invalid = run(r#"{"foreground":[1,2,3]}"#);
    assert_eq!(invalid.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("terminal theme has no background"));
}

fn run(input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tode-theme-bridge"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
