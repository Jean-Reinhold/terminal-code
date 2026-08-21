use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use jsonc_parser::parse_to_serde_value;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{HarnessError, Result};
use crate::scenario::validate_relative;

pub const TARGET_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetManifest {
    pub schema_version: u32,
    pub id: String,
    pub programs: BTreeMap<String, ProgramManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramManifest {
    pub executable: ExecutableSpec,
    #[serde(default)]
    pub args_prefix: Vec<TargetValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExecutableSpec {
    PathLookup { path_lookup: String },
    RepoPath { repo_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TargetValue {
    Literal(String),
    RepoPath { repo_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedProgram {
    pub target_id: String,
    pub program_id: String,
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub args_prefix: Vec<String>,
    pub manifest_sha256: String,
}

pub fn load_target_manifest(path: &Path) -> Result<TargetManifest> {
    let text = fs::read_to_string(path).map_err(|error| {
        HarnessError::io(format!("read target manifest {}", path.display()), error)
    })?;
    let manifest: TargetManifest = parse_to_serde_value(&text, &Default::default())
        .map_err(|error| HarnessError::Json(format!("{}: {error}", path.display())))?;
    validate_manifest(&manifest, path)?;
    Ok(manifest)
}

pub fn resolve_program(
    manifest: &TargetManifest,
    manifest_path: &Path,
    program_id: &str,
    repo_root: &Path,
) -> Result<ResolvedProgram> {
    let program = manifest.programs.get(program_id).ok_or_else(|| {
        HarnessError::Invalid(format!(
            "target {} has no program {program_id}",
            manifest.id
        ))
    })?;
    let repo_root = repo_root.canonicalize().map_err(|error| {
        HarnessError::io(format!("canonicalize {}", repo_root.display()), error)
    })?;
    let executable = match &program.executable {
        ExecutableSpec::PathLookup { path_lookup } => find_executable(path_lookup)?,
        ExecutableSpec::RepoPath { repo_path } => {
            resolve_repo_path(&repo_root, repo_path, manifest_path, "target executable")?
        }
    };
    let mut args_prefix = Vec::with_capacity(program.args_prefix.len());
    for value in &program.args_prefix {
        match value {
            TargetValue::Literal(value) => args_prefix.push(value.clone()),
            TargetValue::RepoPath { repo_path } => {
                validate_relative(repo_path, "target repo path", manifest_path)?;
                let resolved = repo_root.join(repo_path).canonicalize().map_err(|error| {
                    HarnessError::io(format!("canonicalize target repo path {repo_path}"), error)
                })?;
                if !resolved.starts_with(&repo_root) {
                    return Err(HarnessError::Invalid(format!(
                        "target repo path escapes repository: {repo_path}"
                    )));
                }
                args_prefix.push(resolved.to_string_lossy().into_owned());
            }
        }
    }

    Ok(ResolvedProgram {
        target_id: manifest.id.clone(),
        program_id: program_id.to_owned(),
        executable_sha256: sha256_file(&executable)?,
        executable,
        args_prefix,
        manifest_sha256: sha256_file(manifest_path)?,
    })
}

fn validate_manifest(manifest: &TargetManifest, path: &Path) -> Result<()> {
    if manifest.schema_version != TARGET_SCHEMA_VERSION {
        return Err(HarnessError::Invalid(format!(
            "{}: unsupported target schema_version {}",
            path.display(),
            manifest.schema_version
        )));
    }
    validate_id(&manifest.id, "target id", path)?;
    if manifest.programs.is_empty() {
        return Err(HarnessError::Invalid(format!(
            "{}: target must define at least one program",
            path.display()
        )));
    }
    for (id, program) in &manifest.programs {
        validate_id(id, "program id", path)?;
        match &program.executable {
            ExecutableSpec::PathLookup { path_lookup } => {
                if path_lookup.is_empty() || path_lookup.contains('/') || path_lookup.contains('\\')
                {
                    return Err(HarnessError::Invalid(format!(
                        "{}: path_lookup must be a bare executable name",
                        path.display()
                    )));
                }
            }
            ExecutableSpec::RepoPath { repo_path } => {
                validate_relative(repo_path, "target executable", path)?;
            }
        }
        for value in &program.args_prefix {
            if let TargetValue::RepoPath { repo_path } = value {
                validate_relative(repo_path, "target repo path", path)?;
            }
        }
    }
    Ok(())
}

fn resolve_repo_path(
    repo_root: &Path,
    relative: &str,
    manifest_path: &Path,
    label: &str,
) -> Result<PathBuf> {
    validate_relative(relative, label, manifest_path)?;
    let resolved = repo_root
        .join(relative)
        .canonicalize()
        .map_err(|error| HarnessError::io(format!("canonicalize {label} {relative}"), error))?;
    if !resolved.starts_with(repo_root) || !resolved.is_file() {
        return Err(HarnessError::Invalid(format!(
            "{label} must resolve to a repository file: {relative}"
        )));
    }
    Ok(resolved)
}

fn validate_id(value: &str, label: &str, path: &Path) -> Result<()> {
    if value.is_empty()
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
    {
        return Err(HarnessError::Invalid(format!(
            "{}: invalid {label} {value}",
            path.display()
        )));
    }
    Ok(())
}

fn find_executable(name: &str) -> Result<PathBuf> {
    let path = env::var_os("PATH")
        .ok_or_else(|| HarnessError::Invalid("PATH is unavailable for target resolution".into()))?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return candidate.canonicalize().map_err(|error| {
                HarnessError::io(
                    format!("canonicalize executable {}", candidate.display()),
                    error,
                )
            });
        }
    }
    Err(HarnessError::Invalid(format!(
        "target executable not found on PATH: {name}"
    )))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .map_err(|error| HarnessError::io(format!("read {} for hashing", path.display()), error))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}
