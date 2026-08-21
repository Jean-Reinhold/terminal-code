use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{HarnessError, Result};
use crate::scenario::{Risk, Scenario, discover_scenarios, is_contract_id};

#[derive(Debug, Clone, Deserialize)]
pub struct ContractMetadata {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub description: String,
    pub contract_id: String,
    pub status: String,
    pub risk: Risk,
    pub owners: Vec<String>,
    pub surfaces: Vec<String>,
    pub source_paths: Vec<String>,
    #[serde(default)]
    pub scenario_ids: Vec<String>,
    #[serde(default)]
    pub legacy_test_paths: Vec<String>,
    #[serde(default)]
    pub rust_test_paths: Vec<String>,
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Contract {
    pub path: PathBuf,
    pub metadata: ContractMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogSummary {
    pub contracts: usize,
    pub scenarios: usize,
    pub legacy_tests: usize,
    pub rust_tests: usize,
    pub contract_ids: Vec<String>,
    pub scenario_ids: Vec<String>,
}

pub fn check_catalog(
    repo_root: &Path,
    contract_root: &Path,
    scenario_root: &Path,
) -> Result<CatalogSummary> {
    let contracts = discover_contracts(contract_root)?;
    let scenarios = discover_scenarios(scenario_root)?;
    let (legacy_tests, rust_tests) = validate_catalog(repo_root, &contracts, &scenarios)?;
    Ok(CatalogSummary {
        contracts: contracts.len(),
        scenarios: scenarios.len(),
        legacy_tests,
        rust_tests,
        contract_ids: contracts.keys().cloned().collect(),
        scenario_ids: scenarios.keys().cloned().collect(),
    })
}

pub fn discover_contracts(root: &Path) -> Result<BTreeMap<String, Contract>> {
    if !root.exists() {
        return Err(HarnessError::Invalid(format!(
            "contract root does not exist: {}",
            root.display()
        )));
    }
    let mut contracts = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| HarnessError::Invalid(error.to_string()))?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|value| value.to_str()) != Some("md")
        {
            continue;
        }
        if matches!(entry.file_name().to_str(), Some("index.md" | "log.md")) {
            continue;
        }
        let path = entry.into_path();
        let text = fs::read_to_string(&path).map_err(|error| {
            HarnessError::io(format!("read contract {}", path.display()), error)
        })?;
        let frontmatter = extract_frontmatter(&text, &path)?;
        let metadata: ContractMetadata = yaml_serde::from_str(frontmatter)
            .map_err(|error| HarnessError::Yaml(format!("{}: {error}", path.display())))?;
        validate_contract_metadata(&metadata, &path)?;
        let id = metadata.contract_id.clone();
        if let Some(prior) = contracts.insert(
            id.clone(),
            Contract {
                path: path.clone(),
                metadata,
            },
        ) {
            return Err(HarnessError::Invalid(format!(
                "duplicate contract {id} in {} and {}",
                prior.path.display(),
                path.display()
            )));
        }
    }
    if contracts.is_empty() {
        return Err(HarnessError::Invalid(format!(
            "no compatibility contracts found under {}",
            root.display()
        )));
    }
    Ok(contracts)
}

fn validate_catalog(
    repo_root: &Path,
    contracts: &BTreeMap<String, Contract>,
    scenarios: &BTreeMap<String, (PathBuf, Scenario)>,
) -> Result<(usize, usize)> {
    let repo_root = repo_root.canonicalize().map_err(|error| {
        HarnessError::io(format!("canonicalize {}", repo_root.display()), error)
    })?;

    for contract in contracts.values() {
        for source in &contract.metadata.source_paths {
            let source_path = Path::new(source);
            if source_path.is_absolute()
                || source_path
                    .components()
                    .any(|component| !matches!(component, std::path::Component::Normal(_)))
            {
                return Err(HarnessError::Invalid(format!(
                    "{}: invalid source path {source}",
                    contract.path.display()
                )));
            }
            if !repo_root.join(source_path).exists() {
                return Err(HarnessError::Invalid(format!(
                    "{}: source path does not exist: {source}",
                    contract.path.display()
                )));
            }
        }
        for scenario_id in &contract.metadata.scenario_ids {
            let Some((scenario_path, scenario)) = scenarios.get(scenario_id) else {
                return Err(HarnessError::Invalid(format!(
                    "{}: unknown scenario {scenario_id}",
                    contract.path.display()
                )));
            };
            if !scenario.contracts.contains(&contract.metadata.contract_id) {
                return Err(HarnessError::Invalid(format!(
                    "{}: scenario {scenario_id} does not link back to {}",
                    scenario_path.display(),
                    contract.metadata.contract_id
                )));
            }
            if scenario.risk < contract.metadata.risk {
                return Err(HarnessError::Invalid(format!(
                    "{}: scenario {scenario_id} risk is lower than contract {}",
                    scenario_path.display(),
                    contract.metadata.contract_id
                )));
            }
        }
    }

    for (scenario_id, (path, scenario)) in scenarios {
        for contract_id in &scenario.contracts {
            let Some(contract) = contracts.get(contract_id) else {
                return Err(HarnessError::Invalid(format!(
                    "{}: scenario {scenario_id} references unknown contract {contract_id}",
                    path.display()
                )));
            };
            if !contract.metadata.scenario_ids.contains(scenario_id) {
                return Err(HarnessError::Invalid(format!(
                    "{}: contract {contract_id} does not link back to scenario {scenario_id}",
                    contract.path.display()
                )));
            }
        }
    }
    Ok((
        count_mapped_legacy_tests(&repo_root, contracts)?,
        count_mapped_rust_tests(&repo_root, contracts)?,
    ))
}

fn validate_contract_metadata(metadata: &ContractMetadata, path: &Path) -> Result<()> {
    if metadata.kind != "Compatibility Contract" {
        return Err(HarnessError::Invalid(format!(
            "{}: type must be Compatibility Contract",
            path.display()
        )));
    }
    if !is_contract_id(&metadata.contract_id) {
        return Err(HarnessError::Invalid(format!(
            "{}: invalid contract_id {}",
            path.display(),
            metadata.contract_id
        )));
    }
    if metadata.title.trim().is_empty() || metadata.description.trim().is_empty() {
        return Err(HarnessError::Invalid(format!(
            "{}: title and description are required",
            path.display()
        )));
    }
    if !matches!(metadata.status.as_str(), "draft" | "stable" | "deprecated") {
        return Err(HarnessError::Invalid(format!(
            "{}: invalid status {}",
            path.display(),
            metadata.status
        )));
    }
    for (name, values) in [
        ("owners", &metadata.owners),
        ("surfaces", &metadata.surfaces),
        ("source_paths", &metadata.source_paths),
        ("platforms", &metadata.platforms),
    ] {
        validate_nonempty_unique(values, name, path, true)?;
    }
    validate_nonempty_unique(&metadata.scenario_ids, "scenario_ids", path, false)?;
    validate_nonempty_unique(
        &metadata.legacy_test_paths,
        "legacy_test_paths",
        path,
        false,
    )?;
    validate_nonempty_unique(&metadata.rust_test_paths, "rust_test_paths", path, false)?;
    if metadata.status == "stable" && metadata.scenario_ids.is_empty() {
        return Err(HarnessError::Invalid(format!(
            "{}: stable contract requires at least one scenario",
            path.display()
        )));
    }
    Ok(())
}

fn validate_nonempty_unique(
    values: &[String],
    name: &str,
    path: &Path,
    required: bool,
) -> Result<()> {
    if (required && values.is_empty()) || values.iter().any(|value| value.trim().is_empty()) {
        return Err(HarnessError::Invalid(format!(
            "{}: {name} must contain non-empty values",
            path.display()
        )));
    }
    let unique: BTreeSet<_> = values.iter().collect();
    if unique.len() != values.len() {
        return Err(HarnessError::Invalid(format!(
            "{}: {name} contains duplicates",
            path.display()
        )));
    }
    Ok(())
}

fn count_mapped_legacy_tests(
    repo_root: &Path,
    contracts: &BTreeMap<String, Contract>,
) -> Result<usize> {
    let mapped: BTreeSet<_> = contracts
        .values()
        .flat_map(|contract| contract.metadata.legacy_test_paths.iter().cloned())
        .collect();
    for relative in &mapped {
        if !repo_root.join(relative).is_file() {
            return Err(HarnessError::Invalid(format!(
                "mapped legacy test path does not exist: {relative}"
            )));
        }
    }

    let mut count = 0;
    for entry in walkdir::WalkDir::new(repo_root.join("test")).follow_links(false) {
        let entry = entry.map_err(|error| HarnessError::Invalid(error.to_string()))?;
        if !entry.file_type().is_file()
            || !entry.file_name().to_string_lossy().ends_with(".test.js")
        {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(repo_root)
            .map_err(|error| HarnessError::Invalid(error.to_string()))?
            .to_string_lossy()
            .into_owned();
        let text = fs::read_to_string(entry.path()).map_err(|error| {
            HarnessError::io(
                format!("read legacy test {}", entry.path().display()),
                error,
            )
        })?;
        let declarations = count_test_declarations(&text);
        if declarations > 0 && !mapped.contains(&relative) {
            return Err(HarnessError::Invalid(format!(
                "legacy test file has {declarations} unmapped declarations: {relative}"
            )));
        }
        count += declarations;
    }
    Ok(count)
}

fn count_mapped_rust_tests(
    repo_root: &Path,
    contracts: &BTreeMap<String, Contract>,
) -> Result<usize> {
    let mapped: BTreeSet<_> = contracts
        .values()
        .flat_map(|contract| contract.metadata.rust_test_paths.iter().cloned())
        .collect();
    let mut count = 0;
    for relative in mapped {
        let path = repo_root.join(&relative);
        if !path.is_file() {
            return Err(HarnessError::Invalid(format!(
                "mapped Rust test path does not exist: {relative}"
            )));
        }
        let text = fs::read_to_string(&path).map_err(|error| {
            HarnessError::io(format!("read mapped Rust test {relative}"), error)
        })?;
        count += text.match_indices("#[test]").count();
        count += text.match_indices("#[tokio::test]").count();
    }
    Ok(count)
}

fn count_test_declarations(text: &str) -> usize {
    let bytes = text.as_bytes();
    text.match_indices("test")
        .filter(|(start, _)| {
            if *start > 0 {
                let previous = bytes[*start - 1];
                if previous.is_ascii_alphanumeric() || previous == b'_' {
                    return false;
                }
            }
            let mut next = *start + 4;
            while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                next += 1;
            }
            bytes.get(next) == Some(&b'(')
        })
        .count()
}

fn extract_frontmatter<'a>(text: &'a str, path: &Path) -> Result<&'a str> {
    let Some(rest) = text.strip_prefix("---\n") else {
        return Err(HarnessError::Invalid(format!(
            "{}: missing YAML frontmatter",
            path.display()
        )));
    };
    let Some(end) = rest.find("\n---\n") else {
        return Err(HarnessError::Invalid(format!(
            "{}: unterminated YAML frontmatter",
            path.display()
        )));
    };
    Ok(&rest[..end])
}
