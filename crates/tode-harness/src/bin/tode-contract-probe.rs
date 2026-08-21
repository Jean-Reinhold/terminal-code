use std::env;
use std::process::ExitCode;

use tode_core::{parse_goto, resolve_target};

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tode-contract-probe: {error}");
            ExitCode::from(2)
        }
    }
}

fn execute() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mode = args.next().ok_or_else(|| "missing mode".to_owned())?;
    let argument = args
        .next()
        .ok_or_else(|| format!("{mode} needs an argument"))?;
    if args.next().is_some() {
        return Err(format!("{mode} takes one argument"));
    }
    let cwd = env::current_dir().map_err(|error| format!("read current directory: {error}"))?;
    let output = match mode.as_str() {
        "resolve-target" => serde_json::to_string(&resolve_target(Some(&argument), &cwd)),
        "parse-goto" => serde_json::to_string(&parse_goto(&argument, &cwd)),
        _ => return Err(format!("unknown mode {mode}")),
    }
    .map_err(|error| format!("serialize result: {error}"))?;
    println!("{output}");
    Ok(())
}
