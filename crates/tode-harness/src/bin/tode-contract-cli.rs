use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use tode_core::{HELP, installed_version};

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if matches!(flag.as_str(), "--help" | "-h") => {
            if let Err(error) = io::stdout().write_all(HELP.as_bytes()) {
                eprintln!("tode-contract-cli: write help: {error}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        [flag] if matches!(flag.as_str(), "--version" | "-v") => {
            let root = env::var_os("TODE_INSTALL_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            println!("{}", installed_version(&root));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "tode-contract-cli: only --help and --version are available in this contract target"
            );
            ExitCode::from(2)
        }
    }
}
