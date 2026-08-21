use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;
use tode_harness::artifact::{ArtifactRef, ArtifactStore};
use tode_harness::catalog::check_catalog;
use tode_harness::runner::{RunConfig, Verdict, replay, run};
use tode_harness::sandbox::Sandbox;
use tode_harness::scenario::{EnvironmentValue, load_scenario, scenario_schema_json};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn repository_catalog_and_schema_are_current() {
    let root = repository_root();
    let summary = check_catalog(
        &root,
        &root.join(".okf/knowledge/contracts/features"),
        &root.join("harness/scenarios"),
    )
    .unwrap();
    assert_eq!(
        summary.contract_ids,
        (1..=22).map(|id| format!("C{id:02}")).collect::<Vec<_>>()
    );
    assert_eq!(summary.scenarios, 10);
    assert_eq!(summary.legacy_tests, 119);
    assert_eq!(summary.rust_tests, 64);

    let generated: Value = serde_json::from_str(&scenario_schema_json().unwrap()).unwrap();
    let committed: Value = serde_json::from_slice(
        &fs::read(root.join("harness/schemas/scenario-v1.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(committed, generated);
}

#[test]
fn scenario_compiler_rejects_code_and_absolute_fixture_paths() {
    let directory = TempDir::new().unwrap();
    let shell = directory.path().join("shell.scenario.jsonc");
    fs::write(
        &shell,
        valid_scenario(
            r#"{"mode":"single","target":"left"}"#,
            r#"[{"id":"run","kind":"shell","command":"echo unsafe"}]"#,
            r#"[{"kind":"exact.snapshot","observation":"result","snapshot":"baseline.json"}]"#,
            None,
        ),
    )
    .unwrap();
    let shell_error = load_scenario(&shell).unwrap_err().to_string();
    assert!(shell_error.contains("shell"), "{shell_error}");

    let absolute = directory.path().join("absolute.scenario.jsonc");
    fs::write(
        &absolute,
        valid_scenario(
            r#"{"mode":"single","target":"left"}"#,
            process_steps(),
            r#"[{"kind":"exact.snapshot","observation":"result","snapshot":"baseline.json"}]"#,
            Some("/tmp/escape"),
        ),
    )
    .unwrap();
    let path_error = load_scenario(&absolute).unwrap_err().to_string();
    assert!(
        path_error.contains("sandbox fixture must be"),
        "{path_error}"
    );
}

#[test]
fn sandbox_rejects_protected_environment_and_fixture_symlinks() {
    let repo = TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join("fixture")).unwrap();
    fs::write(repo.path().join("outside"), "secret").unwrap();
    symlink("../outside", repo.path().join("fixture/link")).unwrap();
    assert!(Sandbox::create(repo.path(), Some("fixture")).is_err());

    let sandbox = Sandbox::create(repo.path(), None).unwrap();
    let mut environment = BTreeMap::new();
    environment.insert("HOME".to_owned(), EnvironmentValue::Literal("/tmp".into()));
    assert!(sandbox.environment(&environment).is_err());
}

#[test]
fn sandbox_snapshot_records_content_and_excludes_logs() {
    let repo = TempDir::new().unwrap();
    let sandbox = Sandbox::create(repo.path(), None).unwrap();
    fs::write(sandbox.path("workspace/seen.txt").unwrap(), "seen").unwrap();
    fs::write(sandbox.log_path("ignored.log").unwrap(), "ignored").unwrap();
    let artifacts = TempDir::new().unwrap();
    let store = ArtifactStore::new(artifacts.path()).unwrap();
    let snapshot = sandbox.snapshot_tree(&store).unwrap();
    let value: Value = serde_json::from_slice(&store.get(&snapshot).unwrap()).unwrap();
    let entries = value["entries"].as_array().unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| !entry["path"].as_str().unwrap().starts_with("logs"))
    );
    let seen = entries
        .iter()
        .find(|entry| entry["path"] == "workspace/seen.txt")
        .unwrap();
    let content: ArtifactRef = serde_json::from_value(seen["content"].clone()).unwrap();
    assert_eq!(store.get(&content).unwrap(), b"seen");
}

#[test]
fn output_budget_fails_closed() {
    let fixture = RunFixture::new();
    fixture.write_rust_probe_target("left");
    fixture.write_rust_probe_target("right");
    fixture.write_scenario_with_steps(
        r#"{"mode":"differential","left":"left","right":"right"}"#,
        r#"[{"id":"run","kind":"process.exec","program":{"target":"probe"},"args":["emit-bytes","1048577"],"stdin":"closed","capture":["stdout","stderr","exit"],"timeout_ms":1000}]"#,
        r#"[{"kind":"differential.equal","observation":"result"}]"#,
    );

    let error = run(&fixture.config(&["left", "right"]))
        .unwrap_err()
        .to_string();
    assert!(error.contains("exceeds policy maximum"), "{error}");
}

#[test]
fn timed_out_process_groups_are_clean() {
    let fixture = RunFixture::new();
    fixture.write_rust_probe_target("left");
    fixture.write_rust_probe_target("right");
    fixture.write_scenario_with_steps(
        r#"{"mode":"differential","left":"left","right":"right"}"#,
        r#"[{"id":"run","kind":"process.exec","program":{"target":"probe"},"args":["sleep-ms","200"],"stdin":"closed","capture":["stdout","stderr","exit"],"timeout_ms":20}]"#,
        r#"[{"kind":"differential.equal","observation":"result"}]"#,
    );

    let outcome = run(&fixture.config(&["left", "right"])).unwrap();
    assert_eq!(outcome.verdict, Verdict::Passed);
    let observations: Value =
        serde_json::from_slice(&fs::read(outcome.run_directory.join("observations.json")).unwrap())
            .unwrap();
    for observation in observations.as_array().unwrap() {
        assert_eq!(observation["process"]["timed_out"], true);
        assert_eq!(observation["process"]["process_group_clean"], true);
    }
}

#[test]
fn differential_mismatch_is_a_failed_verdict() {
    let fixture = RunFixture::new();
    fixture.write_target("left", "same\n");
    fixture.write_target("right", "different\n");
    fixture.write_scenario(
        r#"{"mode":"differential","left":"left","right":"right"}"#,
        r#"[{"kind":"differential.equal","observation":"result"}]"#,
    );

    let outcome = run(&fixture.config(&["left", "right"])).unwrap();
    assert_eq!(outcome.verdict, Verdict::Failed);
    let assertions: Value =
        serde_json::from_slice(&fs::read(outcome.run_directory.join("assertions.json")).unwrap())
            .unwrap();
    assert_eq!(assertions[0]["passed"], false);
    assert_eq!(assertions[0]["message"], "stdout differs");
}

#[test]
fn replay_passes_then_rejects_corrupted_content_object() {
    let fixture = RunFixture::new();
    fixture.write_target("left", "same\n");
    fs::write(
        fixture.root.path().join("baseline.json"),
        r#"{"exit_code":0,"signal":null,"timed_out":false,"stdout":"same\n","stderr":""}"#,
    )
    .unwrap();
    fixture.write_scenario(
        r#"{"mode":"single","target":"left"}"#,
        r#"[{"kind":"exact.snapshot","observation":"result","snapshot":"baseline.json"}]"#,
    );

    let outcome = run(&fixture.config(&["left"])).unwrap();
    assert_eq!(outcome.verdict, Verdict::Passed);
    let replayed = replay(fixture.root.path(), &fixture.artifact_root, &outcome.run_id).unwrap();
    assert_eq!(replayed.verdict, Verdict::Passed);

    let observations: Value =
        serde_json::from_slice(&fs::read(outcome.run_directory.join("observations.json")).unwrap())
            .unwrap();
    let digest = observations[0]["process"]["stdout_raw"]["sha256"]
        .as_str()
        .unwrap();
    let store = ArtifactStore::new(&fixture.artifact_root).unwrap();
    let object = store.object_path(digest).unwrap();
    fs::write(object, "corrupt").unwrap();
    assert!(replay(fixture.root.path(), &fixture.artifact_root, &outcome.run_id,).is_err());
}

#[test]
fn replay_rejects_tampered_run_plan() {
    let fixture = RunFixture::new();
    fixture.write_target("left", "same\n");
    fs::write(
        fixture.root.path().join("baseline.json"),
        r#"{"exit_code":0,"signal":null,"timed_out":false,"stdout":"same\n","stderr":""}"#,
    )
    .unwrap();
    fixture.write_scenario(
        r#"{"mode":"single","target":"left"}"#,
        r#"[{"kind":"exact.snapshot","observation":"result","snapshot":"baseline.json"}]"#,
    );

    let outcome = run(&fixture.config(&["left"])).unwrap();
    let plan = outcome.run_directory.join("plan.json");
    let mut bytes = fs::read(&plan).unwrap();
    bytes.push(b' ');
    fs::write(plan, bytes).unwrap();
    let error = replay(fixture.root.path(), &fixture.artifact_root, &outcome.run_id)
        .unwrap_err()
        .to_string();
    assert!(error.contains("plan.json expected digest"), "{error}");
}

#[test]
fn policy_rejects_before_artifact_or_sandbox_creation() {
    let fixture = RunFixture::new();
    fixture.write_target("left", "same\n");
    let scenario = valid_scenario(
        r#"{"mode":"single","target":"left"}"#,
        process_steps(),
        r#"[{"kind":"exact.snapshot","observation":"result","snapshot":"baseline.json"}]"#,
        None,
    )
    .replacen(r#""timeout_ms":2000"#, r#""timeout_ms":60001"#, 1);
    fs::write(&fixture.scenario, scenario).unwrap();

    let error = run(&fixture.config(&["left"])).unwrap_err().to_string();
    assert!(error.contains("policy maximum"), "{error}");
    assert!(!fixture.artifact_root.exists());
}

struct RunFixture {
    root: TempDir,
    scenario: PathBuf,
    artifact_root: PathBuf,
}

impl RunFixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let scenario = root.path().join("test.scenario.jsonc");
        let artifact_root = root.path().join("artifacts");
        Self {
            root,
            scenario,
            artifact_root,
        }
    }

    fn write_target(&self, id: &str, output: &str) {
        fs::write(
            self.root.path().join(format!("{id}.target.jsonc")),
            format!(
                r#"{{"schema_version":1,"id":"{id}","programs":{{"probe":{{"executable":{{"path_lookup":"printf"}},"args_prefix":[{}]}}}}}}"#,
                serde_json::to_string(output).unwrap()
            ),
        )
        .unwrap();
    }

    fn write_rust_probe_target(&self, id: &str) {
        let probe = self.root.path().join("probe");
        fs::copy(env!("CARGO_BIN_EXE_tode-contract-probe"), &probe).unwrap();
        fs::write(
            self.root.path().join(format!("{id}.target.jsonc")),
            format!(
                r#"{{"schema_version":1,"id":"{id}","programs":{{"probe":{{"executable":{{"repo_path":"probe"}}}}}}}}"#
            ),
        )
        .unwrap();
    }

    fn write_scenario(&self, targets: &str, assertions: &str) {
        fs::write(
            &self.scenario,
            valid_scenario(targets, process_steps(), assertions, None),
        )
        .unwrap();
    }

    fn write_scenario_with_steps(&self, targets: &str, steps: &str, assertions: &str) {
        fs::write(
            &self.scenario,
            valid_scenario(targets, steps, assertions, None),
        )
        .unwrap();
    }

    fn config(&self, targets: &[&str]) -> RunConfig {
        RunConfig {
            repo_root: self.root.path().to_owned(),
            artifact_root: self.artifact_root.clone(),
            scenario_path: self.scenario.clone(),
            target_manifest_paths: targets
                .iter()
                .map(|id| self.root.path().join(format!("{id}.target.jsonc")))
                .collect(),
        }
    }
}

fn process_steps() -> &'static str {
    r#"[{"id":"run","kind":"process.exec","program":{"target":"probe"},"args":[],"stdin":"closed","capture":["stdout","stderr","exit"],"timeout_ms":1000}]"#
}

fn valid_scenario(targets: &str, steps: &str, assertions: &str, fixture: Option<&str>) -> String {
    let fixture = fixture
        .map(|value| format!(r#","fixture":{}"#, serde_json::to_string(value).unwrap()))
        .unwrap_or_default();
    format!(
        r#"{{
          "$schema":"scenario-v1.schema.json",
          "schema_version":1,
          "id":"test.scenario",
          "title":"test scenario",
          "contracts":["C01"],
          "risk":"medium",
          "targets":{targets},
          "requires":{{"os":["macos","linux"],"capabilities":["process"],"timeout_ms":2000}},
          "sandbox":{{"environment":{{}},"network":"not-required"{fixture}}},
          "steps":{steps},
          "observations":[{{"id":"result","kind":"process.result","from":"run"}}],
          "assertions":{assertions}
        }}"#
    )
}
