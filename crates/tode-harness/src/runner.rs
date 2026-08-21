use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wait_timeout::ChildExt;

use crate::artifact::{ArtifactRef, ArtifactStore, write_json_atomic};
use crate::error::{HarnessError, Result};
use crate::sandbox::Sandbox;
use crate::scenario::{
    AssertionSpec, NormalizationSpec, ObservationSpec, Scenario, Step, TargetSelection,
    canonical_scenario, load_scenario, validate_relative,
};
use crate::target::{
    ResolvedProgram, TargetManifest, load_target_manifest, resolve_program, sha256_file,
};

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub repo_root: PathBuf,
    pub artifact_root: PathBuf,
    pub scenario_path: PathBuf,
    pub target_manifest_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOutcome {
    pub run_id: String,
    pub run_key: String,
    pub scenario_id: String,
    pub verdict: Verdict,
    pub run_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayOutcome {
    pub run_id: String,
    pub verdict: Verdict,
    pub assertions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunManifest {
    schema_version: u32,
    run_id: String,
    run_key: String,
    scenario_id: String,
    scenario_sha256: String,
    targets: Vec<TargetIdentity>,
    observations_sha256: String,
    assertions_sha256: String,
    events_sha256: String,
    evidence_root: String,
    verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct TargetIdentity {
    target_id: String,
    program_id: String,
    executable_sha256: String,
    manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProcessObservation {
    exit_code: Option<i32>,
    signal: Option<i32>,
    timed_out: bool,
    stdout_raw: ArtifactRef,
    stderr_raw: ArtifactRef,
    stdout_normalized: ArtifactRef,
    stderr_normalized: ArtifactRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ObservationRecord {
    target_id: String,
    observation_id: String,
    process: ProcessObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AssertionRecord {
    kind: String,
    observation_id: String,
    expected: Option<ArtifactRef>,
    passed: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedProcess {
    exit_code: Option<i32>,
    signal: Option<i32>,
    timed_out: bool,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunEvent {
    sequence: u64,
    at_unix_ms: u128,
    state: String,
    detail: String,
}

struct EventLog {
    path: PathBuf,
    sequence: u64,
}

impl EventLog {
    fn new(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                HarnessError::io(
                    format!("create event directory {}", parent.display()),
                    error,
                )
            })?;
        }
        File::create(&path).map_err(|error| {
            HarnessError::io(format!("create event log {}", path.display()), error)
        })?;
        Ok(Self { path, sequence: 0 })
    }

    fn append(&mut self, state: &str, detail: impl Into<String>) -> Result<()> {
        self.sequence += 1;
        let event = RunEvent {
            sequence: self.sequence,
            at_unix_ms: now_unix_ms()?,
            state: state.to_owned(),
            detail: detail.into(),
        };
        let mut bytes =
            serde_json::to_vec(&event).map_err(|error| HarnessError::Json(error.to_string()))?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|error| {
                HarnessError::io(format!("open event log {}", self.path.display()), error)
            })?;
        file.write_all(&bytes).map_err(|error| {
            HarnessError::io(format!("append event log {}", self.path.display()), error)
        })?;
        file.sync_data().map_err(|error| {
            HarnessError::io(format!("sync event log {}", self.path.display()), error)
        })?;
        Ok(())
    }
}

pub fn run(config: &RunConfig) -> Result<RunOutcome> {
    let repo_root = config.repo_root.canonicalize().map_err(|error| {
        HarnessError::io(
            format!("canonicalize {}", config.repo_root.display()),
            error,
        )
    })?;
    let scenario = load_scenario(&config.scenario_path)?;
    validate_current_os(&scenario)?;
    let manifests = load_manifests(&config.target_manifest_paths)?;
    let store = ArtifactStore::new(&config.artifact_root)?;
    let scenario_bytes = canonical_scenario(&scenario)?;
    let scenario_ref = store.put(&scenario_bytes, "application/json")?;

    let mut resolved = BTreeMap::new();
    let mut identities = BTreeMap::new();
    for target_id in scenario.targets.ids() {
        let (manifest_path, manifest) = manifests
            .get(target_id)
            .ok_or_else(|| HarnessError::Invalid(format!("missing target manifest {target_id}")))?;
        for step in &scenario.steps {
            let Step::ProcessExec { program, .. } = step;
            let key = (target_id.to_owned(), program.target.clone());
            if resolved.contains_key(&key) {
                continue;
            }
            let program = resolve_program(manifest, manifest_path, &program.target, &repo_root)?;
            identities.insert(
                key.clone(),
                TargetIdentity {
                    target_id: program.target_id.clone(),
                    program_id: program.program_id.clone(),
                    executable_sha256: program.executable_sha256.clone(),
                    manifest_sha256: program.manifest_sha256.clone(),
                },
            );
            resolved.insert(key, program);
        }
    }

    let assertion_expectations = prepare_expectations(&scenario, &repo_root, &store)?;
    let run_key = calculate_run_key(
        &scenario_bytes,
        identities.values(),
        assertion_expectations.iter().flatten(),
    );
    let run_id = format!(
        "{}-{}-{}",
        now_unix_ms()?,
        std::process::id(),
        &run_key[..12]
    );
    let run_directory = store.root().join("runs").join(&run_id);
    fs::create_dir_all(&run_directory).map_err(|error| {
        HarnessError::io(
            format!("create run directory {}", run_directory.display()),
            error,
        )
    })?;
    write_json_atomic(&run_directory.join("scenario.json"), &scenario)?;
    write_json_atomic(
        &run_directory.join("targets.json"),
        &identities.values().cloned().collect::<Vec<_>>(),
    )?;
    let mut events = EventLog::new(run_directory.join("events.jsonl"))?;
    events.append("created", format!("scenario {}", scenario.id))?;

    let mut observations = Vec::new();
    for target_id in scenario.targets.ids() {
        events.append("provisioning", format!("target {target_id}"))?;
        let sandbox = Sandbox::create(&repo_root, scenario.sandbox.fixture.as_deref())?;
        let environment = sandbox.environment(&scenario.sandbox.environment)?;
        let mut step_results = BTreeMap::new();
        events.append("executing", format!("target {target_id}"))?;
        for step in &scenario.steps {
            let Step::ProcessExec { id, program, .. } = step;
            let resolved_program = resolved
                .get(&(target_id.to_owned(), program.target.clone()))
                .expect("resolved program exists");
            let result = execute_process(step, resolved_program, &sandbox, &environment)?;
            step_results.insert(id.clone(), result);
        }
        events.append("observing", format!("target {target_id}"))?;
        for observation in &scenario.observations {
            let ObservationSpec::ProcessResult { id, from } = observation;
            let process = step_results.get(from).ok_or_else(|| {
                HarnessError::Invalid(format!("missing process result for step {from}"))
            })?;
            observations.push(seal_process_observation(
                target_id,
                id,
                process,
                &scenario.normalization,
                &sandbox,
                &store,
            )?);
        }
    }

    events.append("comparing", "evaluate assertions")?;
    let assertion_records =
        evaluate_assertions(&scenario, &observations, &assertion_expectations, &store)?;
    let verdict = if assertion_records.iter().all(|record| record.passed) {
        Verdict::Passed
    } else {
        Verdict::Failed
    };
    write_json_atomic(&run_directory.join("observations.json"), &observations)?;
    write_json_atomic(&run_directory.join("assertions.json"), &assertion_records)?;
    events.append("complete", format!("verdict {verdict:?}"))?;

    let observations_sha256 = sha256_file(&run_directory.join("observations.json"))?;
    let assertions_sha256 = sha256_file(&run_directory.join("assertions.json"))?;
    let events_sha256 = sha256_file(&run_directory.join("events.jsonl"))?;
    let target_identities: Vec<_> = identities.into_values().collect();
    let evidence_root = calculate_evidence_root(
        &scenario_ref.sha256,
        &target_identities,
        &observations_sha256,
        &assertions_sha256,
        &events_sha256,
    );
    let manifest = RunManifest {
        schema_version: 1,
        run_id: run_id.clone(),
        run_key: run_key.clone(),
        scenario_id: scenario.id.clone(),
        scenario_sha256: scenario_ref.sha256,
        targets: target_identities,
        observations_sha256,
        assertions_sha256,
        events_sha256,
        evidence_root,
        verdict: verdict.clone(),
    };
    write_json_atomic(&run_directory.join("run.json"), &manifest)?;

    Ok(RunOutcome {
        run_id,
        run_key,
        scenario_id: scenario.id,
        verdict,
        run_directory,
    })
}

pub fn replay(repo_root: &Path, artifact_root: &Path, run_id: &str) -> Result<ReplayOutcome> {
    let _repo_root = repo_root.canonicalize().map_err(|error| {
        HarnessError::io(format!("canonicalize {}", repo_root.display()), error)
    })?;
    let store = ArtifactStore::new(artifact_root)?;
    let run_directory = store.root().join("runs").join(run_id);
    let manifest: RunManifest = read_json(&run_directory.join("run.json"))?;
    if manifest.run_id != run_id || manifest.schema_version != 1 {
        return Err(HarnessError::Integrity(
            "run manifest identity/version mismatch".into(),
        ));
    }
    let scenario: Scenario = read_json(&run_directory.join("scenario.json"))?;
    let scenario_bytes = canonical_scenario(&scenario)?;
    let scenario_digest = hex::encode(Sha256::digest(&scenario_bytes));
    if scenario_digest != manifest.scenario_sha256 {
        return Err(HarnessError::Integrity(
            "stored scenario digest mismatch".into(),
        ));
    }
    verify_digest(
        &run_directory.join("observations.json"),
        &manifest.observations_sha256,
    )?;
    verify_digest(
        &run_directory.join("assertions.json"),
        &manifest.assertions_sha256,
    )?;
    verify_digest(&run_directory.join("events.jsonl"), &manifest.events_sha256)?;
    let evidence_root = calculate_evidence_root(
        &manifest.scenario_sha256,
        &manifest.targets,
        &manifest.observations_sha256,
        &manifest.assertions_sha256,
        &manifest.events_sha256,
    );
    if evidence_root != manifest.evidence_root {
        return Err(HarnessError::Integrity("evidence root mismatch".into()));
    }

    let observations: Vec<ObservationRecord> = read_json(&run_directory.join("observations.json"))?;
    let stored_assertions: Vec<AssertionRecord> =
        read_json(&run_directory.join("assertions.json"))?;
    for observation in &observations {
        for reference in [
            &observation.process.stdout_raw,
            &observation.process.stderr_raw,
            &observation.process.stdout_normalized,
            &observation.process.stderr_normalized,
        ] {
            store.get(reference)?;
        }
    }
    let expectations: Vec<_> = stored_assertions
        .iter()
        .map(|record| record.expected.clone())
        .collect();
    let replayed = evaluate_assertions(&scenario, &observations, &expectations, &store)?;
    if replayed != stored_assertions {
        return Err(HarnessError::Integrity(
            "replayed assertion records differ from sealed records".into(),
        ));
    }
    let verdict = if replayed.iter().all(|record| record.passed) {
        Verdict::Passed
    } else {
        Verdict::Failed
    };
    if verdict != manifest.verdict {
        return Err(HarnessError::Integrity(
            "replayed verdict differs from manifest".into(),
        ));
    }
    Ok(ReplayOutcome {
        run_id: run_id.to_owned(),
        verdict,
        assertions: replayed.len(),
    })
}

#[derive(Debug)]
struct ProcessResult {
    exit_code: Option<i32>,
    signal: Option<i32>,
    timed_out: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn execute_process(
    step: &Step,
    program: &ResolvedProgram,
    sandbox: &Sandbox,
    environment: &BTreeMap<String, String>,
) -> Result<ProcessResult> {
    let Step::ProcessExec {
        id,
        args,
        timeout_ms,
        ..
    } = step;
    let stdout_path = sandbox.log_path(&format!("{id}.stdout"))?;
    let stderr_path = sandbox.log_path(&format!("{id}.stderr"))?;
    let stdout = File::create(&stdout_path)
        .map_err(|error| HarnessError::io(format!("create {}", stdout_path.display()), error))?;
    let stderr = File::create(&stderr_path)
        .map_err(|error| HarnessError::io(format!("create {}", stderr_path.display()), error))?;

    let mut command = Command::new(&program.executable);
    command.args(&program.args_prefix);
    for arg in args {
        command.arg(sandbox.resolve_value(arg)?);
    }
    command
        .current_dir(sandbox.root())
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .process_group(0);

    let mut child = command.spawn().map_err(|error| {
        HarnessError::Process(format!(
            "spawn target {} program {}: {error}",
            program.target_id, program.program_id
        ))
    })?;
    let pid = child.id() as i32;
    let waited = child
        .wait_timeout(Duration::from_millis(*timeout_ms))
        .map_err(|error| HarnessError::io("wait for target process", error))?;
    let (status, timed_out) = match waited {
        Some(status) => (status, false),
        None => {
            terminate_process_group(pid);
            let status = child
                .wait()
                .map_err(|error| HarnessError::io("wait after target timeout", error))?;
            (status, true)
        }
    };
    terminate_process_group(pid);

    let stdout = fs::read(&stdout_path)
        .map_err(|error| HarnessError::io(format!("read {}", stdout_path.display()), error))?;
    let stderr = fs::read(&stderr_path)
        .map_err(|error| HarnessError::io(format!("read {}", stderr_path.display()), error))?;
    Ok(ProcessResult {
        exit_code: status.code(),
        signal: status.signal(),
        timed_out,
        stdout,
        stderr,
    })
}

fn terminate_process_group(pid: i32) {
    let group = Pid::from_raw(pid);
    let _ = killpg(group, Signal::SIGTERM);
    thread::sleep(Duration::from_millis(20));
    let _ = killpg(group, Signal::SIGKILL);
}

fn seal_process_observation(
    target_id: &str,
    observation_id: &str,
    result: &ProcessResult,
    normalization: &[NormalizationSpec],
    sandbox: &Sandbox,
    store: &ArtifactStore,
) -> Result<ObservationRecord> {
    let stdout_raw = store.put(&result.stdout, "application/octet-stream")?;
    let stderr_raw = store.put(&result.stderr, "application/octet-stream")?;
    let normalize = normalization.iter().any(|entry| {
        entry.observation == observation_id && entry.normalizer == "path.sandbox-root-v1"
    });
    let (stdout_normalized, stderr_normalized) = if normalize {
        (
            store.put(
                &sandbox.normalize_text(&result.stdout)?,
                "text/plain; charset=utf-8",
            )?,
            store.put(
                &sandbox.normalize_text(&result.stderr)?,
                "text/plain; charset=utf-8",
            )?,
        )
    } else {
        (stdout_raw.clone(), stderr_raw.clone())
    };
    Ok(ObservationRecord {
        target_id: target_id.to_owned(),
        observation_id: observation_id.to_owned(),
        process: ProcessObservation {
            exit_code: result.exit_code,
            signal: result.signal,
            timed_out: result.timed_out,
            stdout_raw,
            stderr_raw,
            stdout_normalized,
            stderr_normalized,
        },
    })
}

fn prepare_expectations(
    scenario: &Scenario,
    repo_root: &Path,
    store: &ArtifactStore,
) -> Result<Vec<Option<ArtifactRef>>> {
    let mut expectations = Vec::with_capacity(scenario.assertions.len());
    for assertion in &scenario.assertions {
        match assertion {
            AssertionSpec::ExactSnapshot { snapshot, .. } => {
                validate_relative(snapshot, "snapshot path", &PathBuf::from(&scenario.id))?;
                let path = repo_root.join(snapshot);
                let bytes = fs::read(&path).map_err(|error| {
                    HarnessError::io(format!("read snapshot {}", path.display()), error)
                })?;
                let _: ExpectedProcess = serde_json::from_slice(&bytes)
                    .map_err(|error| HarnessError::Json(format!("{}: {error}", path.display())))?;
                expectations.push(Some(store.put(&bytes, "application/json")?));
            }
            AssertionSpec::DifferentialEqual { .. } => expectations.push(None),
        }
    }
    Ok(expectations)
}

fn evaluate_assertions(
    scenario: &Scenario,
    observations: &[ObservationRecord],
    expectations: &[Option<ArtifactRef>],
    store: &ArtifactStore,
) -> Result<Vec<AssertionRecord>> {
    if expectations.len() != scenario.assertions.len() {
        return Err(HarnessError::Integrity(
            "assertion expectation count mismatch".into(),
        ));
    }
    let mut records = Vec::with_capacity(scenario.assertions.len());
    for (assertion, expected_ref) in scenario.assertions.iter().zip(expectations) {
        match assertion {
            AssertionSpec::ExactSnapshot { observation, .. } => {
                let TargetSelection::Single { target } = &scenario.targets else {
                    unreachable!("scenario validation enforces assertion mode")
                };
                let observed = find_observation(observations, target, observation)?;
                let expected_ref = expected_ref.clone().ok_or_else(|| {
                    HarnessError::Integrity("exact assertion missing expected artifact".into())
                })?;
                let expected: ExpectedProcess = serde_json::from_slice(&store.get(&expected_ref)?)
                    .map_err(|error| HarnessError::Json(error.to_string()))?;
                let actual_stdout = decode(store.get(&observed.process.stdout_normalized)?)?;
                let actual_stderr = decode(store.get(&observed.process.stderr_normalized)?)?;
                let mut differences = Vec::new();
                if observed.process.exit_code != expected.exit_code {
                    differences.push(format!(
                        "exit expected {:?}, got {:?}",
                        expected.exit_code, observed.process.exit_code
                    ));
                }
                if observed.process.signal != expected.signal {
                    differences.push(format!(
                        "signal expected {:?}, got {:?}",
                        expected.signal, observed.process.signal
                    ));
                }
                if observed.process.timed_out != expected.timed_out {
                    differences.push(format!(
                        "timed_out expected {}, got {}",
                        expected.timed_out, observed.process.timed_out
                    ));
                }
                if actual_stdout != expected.stdout {
                    differences.push(format!(
                        "stdout differs (expected {} bytes, got {})",
                        expected.stdout.len(),
                        actual_stdout.len()
                    ));
                }
                if actual_stderr != expected.stderr {
                    differences.push(format!(
                        "stderr differs (expected {} bytes, got {})",
                        expected.stderr.len(),
                        actual_stderr.len()
                    ));
                }
                records.push(AssertionRecord {
                    kind: "exact.snapshot".into(),
                    observation_id: observation.clone(),
                    expected: Some(expected_ref),
                    passed: differences.is_empty(),
                    message: if differences.is_empty() {
                        "exact process snapshot matched".into()
                    } else {
                        differences.join("; ")
                    },
                });
            }
            AssertionSpec::DifferentialEqual { observation } => {
                let TargetSelection::Differential { left, right } = &scenario.targets else {
                    unreachable!("scenario validation enforces assertion mode")
                };
                let left = find_observation(observations, left, observation)?;
                let right = find_observation(observations, right, observation)?;
                let mut differences = Vec::new();
                if left.process.exit_code != right.process.exit_code {
                    differences.push("exit code differs".to_owned());
                }
                if left.process.signal != right.process.signal {
                    differences.push("signal differs".to_owned());
                }
                if left.process.timed_out != right.process.timed_out {
                    differences.push("timeout state differs".to_owned());
                }
                if store.get(&left.process.stdout_normalized)?
                    != store.get(&right.process.stdout_normalized)?
                {
                    differences.push("stdout differs".to_owned());
                }
                if store.get(&left.process.stderr_normalized)?
                    != store.get(&right.process.stderr_normalized)?
                {
                    differences.push("stderr differs".to_owned());
                }
                records.push(AssertionRecord {
                    kind: "differential.equal".into(),
                    observation_id: observation.clone(),
                    expected: None,
                    passed: differences.is_empty(),
                    message: if differences.is_empty() {
                        "differential process observations matched".into()
                    } else {
                        differences.join("; ")
                    },
                });
            }
        }
    }
    Ok(records)
}

fn find_observation<'a>(
    observations: &'a [ObservationRecord],
    target: &str,
    observation: &str,
) -> Result<&'a ObservationRecord> {
    observations
        .iter()
        .find(|record| record.target_id == target && record.observation_id == observation)
        .ok_or_else(|| {
            HarnessError::Integrity(format!(
                "missing observation {observation} for target {target}"
            ))
        })
}

fn load_manifests(paths: &[PathBuf]) -> Result<BTreeMap<String, (PathBuf, TargetManifest)>> {
    let mut manifests = BTreeMap::new();
    for path in paths {
        let manifest = load_target_manifest(path)?;
        let id = manifest.id.clone();
        if let Some((prior, _)) = manifests.insert(id.clone(), (path.clone(), manifest)) {
            return Err(HarnessError::Invalid(format!(
                "duplicate target manifest {id} in {} and {}",
                prior.display(),
                path.display()
            )));
        }
    }
    Ok(manifests)
}

fn validate_current_os(scenario: &Scenario) -> Result<()> {
    let current = std::env::consts::OS;
    if !scenario.requires.os.iter().any(|value| value == current) {
        return Err(HarnessError::Invalid(format!(
            "scenario {} does not support current OS {current}",
            scenario.id
        )));
    }
    Ok(())
}

fn calculate_run_key<'a>(
    scenario: &[u8],
    identities: impl Iterator<Item = &'a TargetIdentity>,
    expectations: impl Iterator<Item = &'a ArtifactRef>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scenario);
    for identity in identities {
        hasher.update(identity.target_id.as_bytes());
        hasher.update(identity.program_id.as_bytes());
        hasher.update(identity.executable_sha256.as_bytes());
        hasher.update(identity.manifest_sha256.as_bytes());
    }
    for expected in expectations {
        hasher.update(expected.sha256.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn calculate_evidence_root(
    scenario_sha256: &str,
    targets: &[TargetIdentity],
    observations_sha256: &str,
    assertions_sha256: &str,
    events_sha256: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scenario_sha256.as_bytes());
    for target in targets {
        hasher.update(target.target_id.as_bytes());
        hasher.update(target.program_id.as_bytes());
        hasher.update(target.executable_sha256.as_bytes());
        hasher.update(target.manifest_sha256.as_bytes());
    }
    hasher.update(observations_sha256.as_bytes());
    hasher.update(assertions_sha256.as_bytes());
    hasher.update(events_sha256.as_bytes());
    hex::encode(hasher.finalize())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = fs::read(path)
        .map_err(|error| HarnessError::io(format!("read JSON {}", path.display()), error))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| HarnessError::Json(format!("{}: {error}", path.display())))
}

fn verify_digest(path: &Path, expected: &str) -> Result<()> {
    let actual = sha256_file(path)?;
    if actual != expected {
        return Err(HarnessError::Integrity(format!(
            "{} expected digest {expected}, got {actual}",
            path.display()
        )));
    }
    Ok(())
}

fn decode(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes)
        .map_err(|error| HarnessError::Integrity(format!("expected UTF-8 artifact: {error}")))
}

fn now_unix_ms() -> Result<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| HarnessError::Invalid(format!("system clock before UNIX epoch: {error}")))
}
