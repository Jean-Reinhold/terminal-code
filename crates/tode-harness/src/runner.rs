use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nix::errno::Errno;
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wait_timeout::ChildExt;

use crate::artifact::{ArtifactRef, ArtifactStore, write_json_atomic};
use crate::error::{HarnessError, Result};
use crate::lease::LeaseBroker;
use crate::sandbox::Sandbox;
use crate::scenario::{
    AssertionSpec, NormalizationSpec, ObservationSpec, Scenario, Step, TargetSelection,
    canonical_scenario, load_scenario, validate_relative,
};
use crate::socket::SocketPeer;
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
    plan_sha256: String,
    targets: Vec<TargetIdentity>,
    observations_sha256: String,
    assertions_sha256: String,
    events_sha256: String,
    evidence_root: String,
    verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RunPolicy {
    max_targets: usize,
    max_steps: usize,
    max_observations: usize,
    max_assertions: usize,
    max_run_timeout_ms: u64,
    max_step_timeout_ms: u64,
    max_output_bytes: u64,
}

impl Default for RunPolicy {
    fn default() -> Self {
        Self {
            max_targets: 2,
            max_steps: 16,
            max_observations: 32,
            max_assertions: 32,
            max_run_timeout_ms: 60_000,
            max_step_timeout_ms: 30_000,
            max_output_bytes: 1_048_576,
        }
    }
}

impl RunPolicy {
    fn validate(&self, scenario: &Scenario) -> Result<()> {
        let targets = scenario.targets.ids().len();
        if targets == 0 || targets > self.max_targets {
            return Err(HarnessError::Invalid(format!(
                "scenario {} has {targets} targets; policy maximum is {}",
                scenario.id, self.max_targets
            )));
        }
        for (name, actual, maximum) in [
            ("steps", scenario.steps.len(), self.max_steps),
            (
                "observations",
                scenario.observations.len(),
                self.max_observations,
            ),
            ("assertions", scenario.assertions.len(), self.max_assertions),
        ] {
            if actual > maximum {
                return Err(HarnessError::Invalid(format!(
                    "scenario {} has {actual} {name}; policy maximum is {maximum}",
                    scenario.id
                )));
            }
        }
        if scenario.requires.timeout_ms > self.max_run_timeout_ms {
            return Err(HarnessError::Invalid(format!(
                "scenario {} timeout {}ms exceeds policy maximum {}ms",
                scenario.id, scenario.requires.timeout_ms, self.max_run_timeout_ms
            )));
        }
        for step in &scenario.steps {
            let (id, timeout_ms) = match step {
                Step::ProcessExec { id, timeout_ms, .. }
                | Step::UnixSocketServer { id, timeout_ms, .. } => (id, timeout_ms),
            };
            if *timeout_ms > self.max_step_timeout_ms {
                return Err(HarnessError::Invalid(format!(
                    "scenario {} step {id} timeout {timeout_ms}ms exceeds policy maximum {}ms",
                    scenario.id, self.max_step_timeout_ms
                )));
            }
        }
        if scenario.retry.is_some() {
            return Err(HarnessError::Invalid(format!(
                "scenario {} declares retry, which execution policy v1 does not implement",
                scenario.id
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RunPlan {
    schema_version: u32,
    run_key: String,
    scenario_id: String,
    scenario: ArtifactRef,
    targets: Vec<TargetIdentity>,
    expectations: Vec<Option<ArtifactRef>>,
    policy: RunPolicy,
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
    sandbox_tree: ArtifactRef,
    process_group_clean: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ObservationRecord {
    target_id: String,
    observation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    process: Option<ProcessObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unix_socket: Option<ArtifactRef>,
}

impl ObservationRecord {
    fn process(&self) -> Result<&ProcessObservation> {
        self.process.as_ref().ok_or_else(|| {
            HarnessError::Integrity(format!(
                "observation {} for target {} is not a process result",
                self.observation_id, self.target_id
            ))
        })
    }

    fn unix_socket(&self) -> Result<&ArtifactRef> {
        self.unix_socket.as_ref().ok_or_else(|| {
            HarnessError::Integrity(format!(
                "observation {} for target {} is not a Unix socket transcript",
                self.observation_id, self.target_id
            ))
        })
    }
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
    let policy = RunPolicy::default();
    policy.validate(&scenario)?;
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
            let Step::ProcessExec { program, .. } = step else {
                continue;
            };
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
    let target_identities: Vec<_> = identities.values().cloned().collect();
    let run_key = calculate_run_key(
        &scenario_bytes,
        target_identities.iter(),
        assertion_expectations.iter().flatten(),
        &policy,
    );
    let plan = RunPlan {
        schema_version: 1,
        run_key: run_key.clone(),
        scenario_id: scenario.id.clone(),
        scenario: scenario_ref.clone(),
        targets: target_identities.clone(),
        expectations: assertion_expectations,
        policy,
    };
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
    write_json_atomic(&run_directory.join("plan.json"), &plan)?;
    let plan_sha256 = sha256_file(&run_directory.join("plan.json"))?;
    write_json_atomic(&run_directory.join("scenario.json"), &scenario)?;
    write_json_atomic(&run_directory.join("targets.json"), &target_identities)?;
    let mut events = EventLog::new(run_directory.join("events.jsonl"))?;
    events.append("created", format!("scenario {}", scenario.id))?;
    events.append("plan_compiled", format!("plan {plan_sha256}"))?;

    let mut observations = Vec::new();
    for target_id in scenario.targets.ids() {
        events.append("provisioning", format!("target {target_id}"))?;
        let sandbox = Sandbox::create(&repo_root, scenario.sandbox.fixture.as_deref())?;
        let mut environment = sandbox.environment(&scenario.sandbox.environment)?;
        let mut lease_broker = LeaseBroker::new()?;
        let mut socket_peers = BTreeMap::new();
        let mut process_results = BTreeMap::new();
        events.append("executing", format!("target {target_id}"))?;
        for step in &scenario.steps {
            match step {
                Step::UnixSocketServer {
                    id,
                    environment: environment_name,
                    reply,
                    max_request_bytes,
                    timeout_ms,
                } => {
                    let peer = SocketPeer::start(
                        &mut lease_broker,
                        id,
                        reply.clone(),
                        *max_request_bytes,
                        Duration::from_millis(*timeout_ms),
                    )?;
                    if environment
                        .insert(
                            environment_name.clone(),
                            peer.path().to_string_lossy().into_owned(),
                        )
                        .is_some()
                    {
                        return Err(HarnessError::Invalid(format!(
                            "socket environment {environment_name} already exists"
                        )));
                    }
                    socket_peers.insert(id.clone(), peer);
                }
                Step::ProcessExec { id, program, .. } => {
                    let resolved_program = resolved
                        .get(&(target_id.to_owned(), program.target.clone()))
                        .expect("resolved program exists");
                    let result = execute_process(
                        step,
                        resolved_program,
                        &sandbox,
                        &environment,
                        plan.policy.max_output_bytes,
                    )?;
                    process_results.insert(id.clone(), result);
                }
            }
        }
        events.append("observing", format!("target {target_id}"))?;
        for observation in &scenario.observations {
            match observation {
                ObservationSpec::ProcessResult { id, from } => {
                    let process = process_results.get(from).ok_or_else(|| {
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
                ObservationSpec::UnixSocketTranscript { id, from } => {
                    let peer = socket_peers.remove(from).ok_or_else(|| {
                        HarnessError::Invalid(format!("missing socket peer for step {from}"))
                    })?;
                    let transcript = peer.finish()?;
                    let bytes = serde_json::to_vec(&transcript)
                        .map_err(|error| HarnessError::Json(error.to_string()))?;
                    observations.push(ObservationRecord {
                        target_id: target_id.to_owned(),
                        observation_id: id.clone(),
                        process: None,
                        unix_socket: Some(store.put(&bytes, "application/json")?),
                    });
                }
            }
        }
        if !socket_peers.is_empty() {
            return Err(HarnessError::Invalid(format!(
                "target {target_id} has unobserved Unix socket peers"
            )));
        }
    }

    events.append("comparing", "evaluate assertions")?;
    let assertion_records =
        evaluate_assertions(&scenario, &observations, &plan.expectations, &store)?;
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
    let evidence_root = calculate_evidence_root(
        &scenario_ref.sha256,
        &plan_sha256,
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
        plan_sha256,
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
    verify_digest(&run_directory.join("plan.json"), &manifest.plan_sha256)?;
    let plan: RunPlan = read_json(&run_directory.join("plan.json"))?;
    if plan.schema_version != 1
        || plan.run_key != manifest.run_key
        || plan.scenario_id != manifest.scenario_id
        || plan.scenario.sha256 != manifest.scenario_sha256
        || plan.targets != manifest.targets
    {
        return Err(HarnessError::Integrity(
            "run plan and manifest identity mismatch".into(),
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
    if store.get(&plan.scenario)? != scenario_bytes {
        return Err(HarnessError::Integrity(
            "scenario artifact differs from stored scenario".into(),
        ));
    }
    plan.policy.validate(&scenario)?;
    let replayed_run_key = calculate_run_key(
        &scenario_bytes,
        plan.targets.iter(),
        plan.expectations.iter().flatten(),
        &plan.policy,
    );
    if replayed_run_key != plan.run_key {
        return Err(HarnessError::Integrity(
            "recomputed run key differs from plan".into(),
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
        &manifest.plan_sha256,
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
        match (&observation.process, &observation.unix_socket) {
            (Some(process), None) => {
                for reference in [
                    &process.stdout_raw,
                    &process.stderr_raw,
                    &process.stdout_normalized,
                    &process.stderr_normalized,
                    &process.sandbox_tree,
                ] {
                    store.get(reference)?;
                }
            }
            (None, Some(transcript)) => {
                store.get(transcript)?;
            }
            _ => {
                return Err(HarnessError::Integrity(format!(
                    "observation {} has invalid variant fields",
                    observation.observation_id
                )));
            }
        }
    }
    for expected in plan.expectations.iter().flatten() {
        store.get(expected)?;
    }
    let replayed = evaluate_assertions(&scenario, &observations, &plan.expectations, &store)?;
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
    process_group_clean: bool,
}

fn execute_process(
    step: &Step,
    program: &ResolvedProgram,
    sandbox: &Sandbox,
    environment: &BTreeMap<String, String>,
    max_output_bytes: u64,
) -> Result<ProcessResult> {
    let Step::ProcessExec {
        id,
        args,
        timeout_ms,
        ..
    } = step
    else {
        return Err(HarnessError::Integrity(
            "process adapter received a non-process step".into(),
        ));
    };
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
            signal_process_group(pid);
            let status = child
                .wait()
                .map_err(|error| HarnessError::io("wait after target timeout", error))?;
            (status, true)
        }
    };
    let process_group_clean = terminate_process_group(pid);

    let stdout_bytes = fs::metadata(&stdout_path)
        .map_err(|error| HarnessError::io(format!("stat {}", stdout_path.display()), error))?
        .len();
    let stderr_bytes = fs::metadata(&stderr_path)
        .map_err(|error| HarnessError::io(format!("stat {}", stderr_path.display()), error))?
        .len();
    if stdout_bytes.saturating_add(stderr_bytes) > max_output_bytes {
        return Err(HarnessError::Invalid(format!(
            "process output {} bytes exceeds policy maximum {max_output_bytes} bytes",
            stdout_bytes.saturating_add(stderr_bytes)
        )));
    }
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
        process_group_clean,
    })
}

fn signal_process_group(pid: i32) {
    let group = Pid::from_raw(pid);
    let _ = killpg(group, Signal::SIGTERM);
    thread::sleep(Duration::from_millis(20));
    let _ = killpg(group, Signal::SIGKILL);
}

fn terminate_process_group(pid: i32) -> bool {
    signal_process_group(pid);
    let group = Pid::from_raw(pid);
    for _ in 0..10 {
        match killpg(group, None) {
            Err(Errno::ESRCH) => return true,
            _ => thread::sleep(Duration::from_millis(10)),
        }
    }
    false
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
    let sandbox_tree = sandbox.snapshot_tree(store)?;
    Ok(ObservationRecord {
        target_id: target_id.to_owned(),
        observation_id: observation_id.to_owned(),
        process: Some(ProcessObservation {
            exit_code: result.exit_code,
            signal: result.signal,
            timed_out: result.timed_out,
            stdout_raw,
            stderr_raw,
            stdout_normalized,
            stderr_normalized,
            sandbox_tree,
            process_group_clean: result.process_group_clean,
        }),
        unix_socket: None,
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
            AssertionSpec::ExactJson { snapshot, .. } => {
                validate_relative(snapshot, "snapshot path", &PathBuf::from(&scenario.id))?;
                let path = repo_root.join(snapshot);
                let bytes = fs::read(&path).map_err(|error| {
                    HarnessError::io(format!("read snapshot {}", path.display()), error)
                })?;
                let _: serde_json::Value = serde_json::from_slice(&bytes)
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
    let mut records = Vec::with_capacity(scenario.assertions.len() + observations.len());
    for (assertion, expected_ref) in scenario.assertions.iter().zip(expectations) {
        match assertion {
            AssertionSpec::ExactSnapshot { observation, .. } => {
                let TargetSelection::Single { target } = &scenario.targets else {
                    unreachable!("scenario validation enforces assertion mode")
                };
                let observed = find_observation(observations, target, observation)?;
                let process = observed.process()?;
                let expected_ref = expected_ref.clone().ok_or_else(|| {
                    HarnessError::Integrity("exact assertion missing expected artifact".into())
                })?;
                let expected: ExpectedProcess = serde_json::from_slice(&store.get(&expected_ref)?)
                    .map_err(|error| HarnessError::Json(error.to_string()))?;
                let actual_stdout = decode(store.get(&process.stdout_normalized)?)?;
                let actual_stderr = decode(store.get(&process.stderr_normalized)?)?;
                let mut differences = Vec::new();
                if process.exit_code != expected.exit_code {
                    differences.push(format!(
                        "exit expected {:?}, got {:?}",
                        expected.exit_code, process.exit_code
                    ));
                }
                if process.signal != expected.signal {
                    differences.push(format!(
                        "signal expected {:?}, got {:?}",
                        expected.signal, process.signal
                    ));
                }
                if process.timed_out != expected.timed_out {
                    differences.push(format!(
                        "timed_out expected {}, got {}",
                        expected.timed_out, process.timed_out
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
            AssertionSpec::ExactJson { observation, .. } => {
                let TargetSelection::Single { target } = &scenario.targets else {
                    unreachable!("scenario validation enforces assertion mode")
                };
                let observed = find_observation(observations, target, observation)?;
                let expected_ref = expected_ref.clone().ok_or_else(|| {
                    HarnessError::Integrity("exact JSON assertion missing expected artifact".into())
                })?;
                let expected: serde_json::Value =
                    serde_json::from_slice(&store.get(&expected_ref)?)
                        .map_err(|error| HarnessError::Json(error.to_string()))?;
                let actual: serde_json::Value =
                    serde_json::from_slice(&store.get(observed.unix_socket()?)?)
                        .map_err(|error| HarnessError::Json(error.to_string()))?;
                let passed = actual == expected;
                records.push(AssertionRecord {
                    kind: "exact.json".into(),
                    observation_id: observation.clone(),
                    expected: Some(expected_ref),
                    passed,
                    message: if passed {
                        "exact JSON snapshot matched".into()
                    } else {
                        "JSON snapshot differs".into()
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
                match (
                    &left.process,
                    &right.process,
                    &left.unix_socket,
                    &right.unix_socket,
                ) {
                    (Some(left), Some(right), None, None) => {
                        if left.exit_code != right.exit_code {
                            differences.push("exit code differs".to_owned());
                        }
                        if left.signal != right.signal {
                            differences.push("signal differs".to_owned());
                        }
                        if left.timed_out != right.timed_out {
                            differences.push("timeout state differs".to_owned());
                        }
                        if store.get(&left.stdout_normalized)?
                            != store.get(&right.stdout_normalized)?
                        {
                            differences.push("stdout differs".to_owned());
                        }
                        if store.get(&left.stderr_normalized)?
                            != store.get(&right.stderr_normalized)?
                        {
                            differences.push("stderr differs".to_owned());
                        }
                        if store.get(&left.sandbox_tree)? != store.get(&right.sandbox_tree)? {
                            differences.push("filesystem tree differs".to_owned());
                        }
                        if left.process_group_clean != right.process_group_clean {
                            differences.push("process-group cleanup differs".to_owned());
                        }
                    }
                    (None, None, Some(left), Some(right)) => {
                        if store.get(left)? != store.get(right)? {
                            differences.push("Unix socket transcript differs".to_owned());
                        }
                    }
                    _ => differences.push("observation types differ".to_owned()),
                }
                records.push(AssertionRecord {
                    kind: "differential.equal".into(),
                    observation_id: observation.clone(),
                    expected: None,
                    passed: differences.is_empty(),
                    message: if differences.is_empty() {
                        "differential observations matched".into()
                    } else {
                        differences.join("; ")
                    },
                });
            }
        }
    }
    for observation in observations {
        let Some(process) = &observation.process else {
            continue;
        };
        records.push(AssertionRecord {
            kind: "invariant.process-group-clean".into(),
            observation_id: observation.observation_id.clone(),
            expected: None,
            passed: process.process_group_clean,
            message: if process.process_group_clean {
                format!("target {} process group is clean", observation.target_id)
            } else {
                format!("target {} leaked its process group", observation.target_id)
            },
        });
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
    policy: &RunPolicy,
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
    for value in [
        policy.max_targets as u64,
        policy.max_steps as u64,
        policy.max_observations as u64,
        policy.max_assertions as u64,
        policy.max_run_timeout_ms,
        policy.max_step_timeout_ms,
        policy.max_output_bytes,
    ] {
        hasher.update(value.to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

fn calculate_evidence_root(
    scenario_sha256: &str,
    plan_sha256: &str,
    targets: &[TargetIdentity],
    observations_sha256: &str,
    assertions_sha256: &str,
    events_sha256: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scenario_sha256.as_bytes());
    hasher.update(plan_sha256.as_bytes());
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
