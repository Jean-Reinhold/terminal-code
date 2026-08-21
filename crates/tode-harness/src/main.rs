use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tode_harness::artifact::write_json_atomic;
use tode_harness::catalog::check_catalog;
use tode_harness::runner::{RunConfig, Verdict, replay, run};
use tode_harness::scenario::scenario_schema_json;
use tode_harness::{HarnessError, Result};

#[derive(Debug, Parser)]
#[command(
    name = "tode-harness",
    version,
    about = "Deterministic compatibility harness for terminal-code"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Catalog {
        #[command(subcommand)]
        command: CatalogCommand,
    },
    Schema {
        #[arg(long, default_value = "harness/schemas/scenario-v1.schema.json")]
        output: PathBuf,
    },
    Run {
        #[arg(long)]
        scenario: PathBuf,
        #[arg(long = "target-manifest", required = true)]
        target_manifests: Vec<PathBuf>,
        #[arg(long, default_value = ".harness-artifacts")]
        artifact_root: PathBuf,
        #[arg(long, default_value = ".")]
        repo_root: PathBuf,
    },
    Replay {
        run_id: String,
        #[arg(long, default_value = ".harness-artifacts")]
        artifact_root: PathBuf,
        #[arg(long, default_value = ".")]
        repo_root: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum CatalogCommand {
    Check {
        #[arg(long, default_value = ".okf/knowledge/contracts/features")]
        contract_root: PathBuf,
        #[arg(long, default_value = "harness/scenarios")]
        scenario_root: PathBuf,
        #[arg(long, default_value = ".")]
        repo_root: PathBuf,
    },
}

fn main() {
    let exit = match execute(Cli::parse()) {
        Ok(exit) => exit,
        Err(error) => {
            eprintln!("tode-harness: {error}");
            2
        }
    };
    std::process::exit(exit);
}

fn execute(cli: Cli) -> Result<i32> {
    match cli.command {
        Command::Catalog { command } => match command {
            CatalogCommand::Check {
                contract_root,
                scenario_root,
                repo_root,
            } => {
                let summary = check_catalog(&repo_root, &contract_root, &scenario_root)?;
                println!(
                    "catalog ok: {} contracts, {} scenarios, {} mapped legacy tests ({})",
                    summary.contracts,
                    summary.scenarios,
                    summary.legacy_tests,
                    summary.contract_ids.join(", ")
                );
                Ok(0)
            }
        },
        Command::Schema { output } => {
            let schema = scenario_schema_json()?;
            let value: serde_json::Value = serde_json::from_str(&schema)
                .map_err(|error| HarnessError::Json(error.to_string()))?;
            write_json_atomic(&output, &value)?;
            println!("wrote {}", output.display());
            Ok(0)
        }
        Command::Run {
            scenario,
            target_manifests,
            artifact_root,
            repo_root,
        } => {
            let outcome = run(&RunConfig {
                repo_root,
                artifact_root,
                scenario_path: scenario,
                target_manifest_paths: target_manifests,
            })?;
            println!(
                "run {}: {:?} ({})",
                outcome.run_id,
                outcome.verdict,
                outcome.run_directory.display()
            );
            Ok(if outcome.verdict == Verdict::Passed {
                0
            } else {
                1
            })
        }
        Command::Replay {
            run_id,
            artifact_root,
            repo_root,
        } => {
            let outcome = replay(&repo_root, &artifact_root, &run_id)?;
            println!(
                "replay {}: {:?} ({} assertions)",
                outcome.run_id, outcome.verdict, outcome.assertions
            );
            Ok(if outcome.verdict == Verdict::Passed {
                0
            } else {
                1
            })
        }
    }
}
