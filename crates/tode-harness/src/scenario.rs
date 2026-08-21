use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use jsonc_parser::parse_to_serde_value;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

use crate::error::{HarnessError, Result};

pub const SCENARIO_SCHEMA_VERSION: u32 = 1;
pub const SCENARIO_SCHEMA_FILE: &str = "scenario-v1.schema.json";

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub contracts: Vec<String>,
    pub risk: Risk,
    pub targets: TargetSelection,
    pub requires: Requirements,
    pub sandbox: SandboxSpec,
    pub steps: Vec<Step>,
    pub observations: Vec<ObservationSpec>,
    #[serde(default)]
    pub normalization: Vec<NormalizationSpec>,
    pub assertions: Vec<AssertionSpec>,
    #[serde(default)]
    pub retry: Option<RetrySpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum TargetSelection {
    Single { target: String },
    Differential { left: String, right: String },
}

impl TargetSelection {
    pub fn ids(&self) -> Vec<&str> {
        match self {
            Self::Single { target } => vec![target],
            Self::Differential { left, right } => vec![left, right],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    pub os: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SandboxSpec {
    #[serde(default)]
    pub fixture: Option<String>,
    #[serde(default)]
    pub environment: BTreeMap<String, EnvironmentValue>,
    pub network: NetworkMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkMode {
    NotRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum EnvironmentValue {
    Literal(String),
    SandboxPath { sandbox_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Step {
    #[serde(rename = "process.exec")]
    ProcessExec {
        id: String,
        program: ProgramRef,
        #[serde(default)]
        args: Vec<ScenarioValue>,
        stdin: StdinMode,
        capture: Vec<ProcessCapture>,
        timeout_ms: u64,
    },
    #[serde(rename = "unix_socket.server")]
    UnixSocketServer {
        id: String,
        environment: String,
        reply: serde_json::Value,
        max_request_bytes: u64,
        timeout_ms: u64,
    },
}

impl Step {
    pub fn id(&self) -> &str {
        match self {
            Self::ProcessExec { id, .. } | Self::UnixSocketServer { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramRef {
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ScenarioValue {
    Literal(String),
    SandboxPath { sandbox_path: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum StdinMode {
    Closed,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum ProcessCapture {
    Stdout,
    Stderr,
    Exit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ObservationSpec {
    #[serde(rename = "process.result")]
    ProcessResult { id: String, from: String },
    #[serde(rename = "unix_socket.transcript")]
    UnixSocketTranscript { id: String, from: String },
}

impl ObservationSpec {
    pub fn id(&self) -> &str {
        match self {
            Self::ProcessResult { id, .. } | Self::UnixSocketTranscript { id, .. } => id,
        }
    }

    pub fn source_step(&self) -> &str {
        match self {
            Self::ProcessResult { from, .. } | Self::UnixSocketTranscript { from, .. } => from,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NormalizationSpec {
    pub observation: String,
    pub normalizer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum AssertionSpec {
    #[serde(rename = "exact.snapshot")]
    ExactSnapshot {
        observation: String,
        snapshot: String,
    },
    #[serde(rename = "exact.json")]
    ExactJson {
        observation: String,
        snapshot: String,
    },
    #[serde(rename = "differential.equal")]
    DifferentialEqual { observation: String },
}

impl AssertionSpec {
    pub fn observation(&self) -> &str {
        match self {
            Self::ExactSnapshot { observation, .. }
            | Self::ExactJson { observation, .. }
            | Self::DifferentialEqual { observation } => observation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrySpec {
    pub max_attempts: u8,
    pub only: Vec<RetryClass>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RetryClass {
    WorkerLost,
    ArtifactStoreUnavailable,
}

pub fn load_scenario(path: &Path) -> Result<Scenario> {
    let text = fs::read_to_string(path)
        .map_err(|error| HarnessError::io(format!("read scenario {}", path.display()), error))?;
    let scenario: Scenario = parse_to_serde_value(&text, &Default::default())
        .map_err(|error| HarnessError::Json(format!("{}: {error}", path.display())))?;
    validate_scenario(&scenario, path)?;
    Ok(scenario)
}

pub fn scenario_schema_json() -> Result<String> {
    serde_json::to_string_pretty(&schema_for!(Scenario))
        .map(|json| format!("{json}\n"))
        .map_err(|error| HarnessError::Json(error.to_string()))
}

pub fn canonical_scenario(scenario: &Scenario) -> Result<Vec<u8>> {
    serde_json::to_vec(scenario).map_err(|error| HarnessError::Json(error.to_string()))
}

pub fn discover_scenarios(root: &Path) -> Result<BTreeMap<String, (PathBuf, Scenario)>> {
    let mut scenarios = BTreeMap::new();
    if !root.exists() {
        return Err(HarnessError::Invalid(format!(
            "scenario root does not exist: {}",
            root.display()
        )));
    }

    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| HarnessError::Invalid(error.to_string()))?;
        if !entry.file_type().is_file()
            || !entry
                .file_name()
                .to_string_lossy()
                .ends_with(".scenario.jsonc")
        {
            continue;
        }
        let path = entry.into_path();
        let scenario = load_scenario(&path)?;
        if let Some((prior, _)) = scenarios.insert(scenario.id.clone(), (path.clone(), scenario)) {
            return Err(HarnessError::Invalid(format!(
                "duplicate scenario id in {} and {}",
                prior.display(),
                path.display()
            )));
        }
    }
    Ok(scenarios)
}

fn validate_scenario(scenario: &Scenario, path: &Path) -> Result<()> {
    if scenario.schema_version != SCENARIO_SCHEMA_VERSION {
        return invalid(
            path,
            format!("unsupported schema_version {}", scenario.schema_version),
        );
    }
    if !scenario.schema.ends_with(SCENARIO_SCHEMA_FILE) {
        return invalid(
            path,
            format!("$schema must reference {SCENARIO_SCHEMA_FILE}"),
        );
    }
    validate_identifier(&scenario.id, "scenario id", path)?;
    if scenario.title.trim().is_empty() {
        return invalid(path, "title must not be empty");
    }
    if scenario.contracts.is_empty() {
        return invalid(path, "at least one contract is required");
    }
    for contract in &scenario.contracts {
        if !is_contract_id(contract) {
            return invalid(path, format!("invalid contract id {contract}"));
        }
    }
    if scenario.requires.timeout_ms == 0 {
        return invalid(path, "requires.timeout_ms must be positive");
    }
    let supported_capabilities = ["filesystem", "process", "unix-socket"];
    for capability in &scenario.requires.capabilities {
        if !supported_capabilities.contains(&capability.as_str()) {
            return invalid(path, format!("unsupported capability {capability}"));
        }
    }
    if let Some(fixture) = &scenario.sandbox.fixture {
        validate_relative(fixture, "sandbox fixture", path)?;
    }
    for value in scenario.sandbox.environment.values() {
        if let EnvironmentValue::SandboxPath { sandbox_path } = value {
            validate_relative(sandbox_path, "environment sandbox path", path)?;
        }
    }

    let target_ids = scenario.targets.ids();
    if target_ids.iter().any(|id| id.trim().is_empty()) {
        return invalid(path, "target ids must not be empty");
    }
    if matches!(&scenario.targets, TargetSelection::Differential { left, right } if left == right) {
        return invalid(path, "differential targets must be distinct");
    }

    let mut step_ids = BTreeSet::new();
    let mut socket_environments = BTreeSet::new();
    let mut seen_process = false;
    for step in &scenario.steps {
        validate_identifier(step.id(), "step id", path)?;
        if !step_ids.insert(step.id()) {
            return invalid(path, format!("duplicate step id {}", step.id()));
        }
        match step {
            Step::ProcessExec {
                program,
                args,
                capture,
                timeout_ms,
                ..
            } => {
                seen_process = true;
                validate_identifier(&program.target, "program target", path)?;
                if *timeout_ms == 0 || *timeout_ms > scenario.requires.timeout_ms {
                    return invalid(
                        path,
                        "process timeout must be positive and within run timeout",
                    );
                }
                for arg in args {
                    if let ScenarioValue::SandboxPath { sandbox_path } = arg {
                        validate_relative(sandbox_path, "argument sandbox path", path)?;
                    }
                }
                let captures: BTreeSet<_> = capture.iter().copied().collect();
                for required in [
                    ProcessCapture::Stdout,
                    ProcessCapture::Stderr,
                    ProcessCapture::Exit,
                ] {
                    if !captures.contains(&required) {
                        return invalid(
                            path,
                            "process capture must include stdout, stderr, and exit",
                        );
                    }
                }
            }
            Step::UnixSocketServer {
                environment,
                max_request_bytes,
                timeout_ms,
                ..
            } => {
                if seen_process {
                    return invalid(path, "socket server steps must precede process steps");
                }
                validate_environment_name(environment, path)?;
                if scenario.sandbox.environment.contains_key(environment)
                    || !socket_environments.insert(environment)
                {
                    return invalid(path, format!("duplicate socket environment {environment}"));
                }
                if *max_request_bytes == 0 || *max_request_bytes > 65_536 {
                    return invalid(path, "socket max_request_bytes must be between 1 and 65536");
                }
                if *timeout_ms == 0 || *timeout_ms > scenario.requires.timeout_ms {
                    return invalid(
                        path,
                        "socket timeout must be positive and within run timeout",
                    );
                }
            }
        }
    }
    if step_ids.is_empty() {
        return invalid(path, "at least one step is required");
    }

    let mut observation_ids = BTreeSet::new();
    for observation in &scenario.observations {
        validate_identifier(observation.id(), "observation id", path)?;
        if !observation_ids.insert(observation.id()) {
            return invalid(
                path,
                format!("duplicate observation id {}", observation.id()),
            );
        }
        let Some(source) = scenario
            .steps
            .iter()
            .find(|step| step.id() == observation.source_step())
        else {
            return invalid(
                path,
                format!("unknown source step {}", observation.source_step()),
            );
        };
        if !matches!(
            (observation, source),
            (
                ObservationSpec::ProcessResult { .. },
                Step::ProcessExec { .. }
            ) | (
                ObservationSpec::UnixSocketTranscript { .. },
                Step::UnixSocketServer { .. }
            )
        ) {
            return invalid(
                path,
                format!(
                    "observation {} has incompatible source step {}",
                    observation.id(),
                    observation.source_step()
                ),
            );
        }
    }
    if observation_ids.is_empty() {
        return invalid(path, "at least one observation is required");
    }
    for step in &scenario.steps {
        if matches!(step, Step::UnixSocketServer { .. }) {
            let transcripts = scenario
                .observations
                .iter()
                .filter(|observation| {
                    matches!(
                        observation,
                        ObservationSpec::UnixSocketTranscript { from, .. } if from == step.id()
                    )
                })
                .count();
            if transcripts != 1 {
                return invalid(
                    path,
                    format!(
                        "socket server step {} requires exactly one transcript observation",
                        step.id()
                    ),
                );
            }
        }
    }

    for normalization in &scenario.normalization {
        let Some(observation) = scenario
            .observations
            .iter()
            .find(|observation| observation.id() == normalization.observation)
        else {
            return invalid(
                path,
                format!(
                    "unknown normalized observation {}",
                    normalization.observation
                ),
            );
        };
        if normalization.normalizer != "path.sandbox-root-v1" {
            return invalid(
                path,
                format!("unknown normalizer {}", normalization.normalizer),
            );
        }
        if matches!(observation, ObservationSpec::UnixSocketTranscript { .. }) {
            return invalid(path, "path normalizer does not apply to socket transcript");
        }
    }

    if scenario.assertions.is_empty() {
        return invalid(path, "at least one assertion is required");
    }
    for assertion in &scenario.assertions {
        if !observation_ids.contains(assertion.observation()) {
            return invalid(
                path,
                format!("unknown asserted observation {}", assertion.observation()),
            );
        }
        match assertion {
            AssertionSpec::ExactSnapshot { snapshot, .. } => {
                validate_relative(snapshot, "snapshot path", path)?;
                if !matches!(scenario.targets, TargetSelection::Single { .. }) {
                    return invalid(path, "exact.snapshot requires single target mode");
                }
            }
            AssertionSpec::ExactJson { snapshot, .. } => {
                validate_relative(snapshot, "snapshot path", path)?;
                if !matches!(scenario.targets, TargetSelection::Single { .. }) {
                    return invalid(path, "exact.json requires single target mode");
                }
            }
            AssertionSpec::DifferentialEqual { .. } => {
                if !matches!(scenario.targets, TargetSelection::Differential { .. }) {
                    return invalid(path, "differential.equal requires differential mode");
                }
            }
        }
    }

    if let Some(retry) = &scenario.retry {
        if retry.max_attempts == 0 || retry.max_attempts > 3 {
            return invalid(path, "retry.max_attempts must be between 1 and 3");
        }
        if retry.only.is_empty() {
            return invalid(path, "retry.only must not be empty");
        }
    }
    Ok(())
}

pub fn is_contract_id(value: &str) -> bool {
    value.len() == 3
        && value.starts_with('C')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_digit)
}

pub fn validate_relative(value: &str, label: &str, path: &Path) -> Result<()> {
    let candidate = Path::new(value);
    if candidate.as_os_str().is_empty() || candidate.is_absolute() {
        return invalid(path, format!("{label} must be a non-empty relative path"));
    }
    if candidate
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return invalid(path, format!("{label} contains a forbidden path component"));
    }
    Ok(())
}

fn validate_identifier(value: &str, label: &str, path: &Path) -> Result<()> {
    if value.is_empty()
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
    {
        return invalid(path, format!("{label} has invalid characters: {value}"));
    }
    Ok(())
}

fn validate_environment_name(value: &str, path: &Path) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        || matches!(
            value,
            "HOME"
                | "PATH"
                | "XDG_DATA_HOME"
                | "XDG_STATE_HOME"
                | "XDG_CACHE_HOME"
                | "XDG_CONFIG_HOME"
                | "XDG_BIN_HOME"
                | "TODE_INSTALL_ROOT"
        )
        || value.starts_with("DYLD_")
        || value.starts_with("LD_")
    {
        return invalid(path, format!("unsafe socket environment name: {value}"));
    }
    Ok(())
}

fn invalid<T>(path: &Path, message: impl Into<String>) -> Result<T> {
    Err(HarnessError::Invalid(format!(
        "{}: {}",
        path.display(),
        message.into()
    )))
}
