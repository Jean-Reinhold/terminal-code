use std::env;
use std::io::{self, Write};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

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
    let arguments: Vec<_> = args.collect();
    let cwd = env::current_dir().map_err(|error| format!("read current directory: {error}"))?;
    match mode.as_str() {
        "resolve-target" => {
            let argument = one_argument(&mode, &arguments)?;
            println!(
                "{}",
                serde_json::to_string(&resolve_target(Some(argument), &cwd))
                    .map_err(|error| format!("serialize result: {error}"))?
            );
        }
        "parse-goto" => {
            let argument = one_argument(&mode, &arguments)?;
            println!(
                "{}",
                serde_json::to_string(&parse_goto(argument, &cwd))
                    .map_err(|error| format!("serialize result: {error}"))?
            );
        }
        "sleep-ms" => {
            let milliseconds = one_argument(&mode, &arguments)?
                .parse::<u64>()
                .map_err(|error| format!("parse milliseconds: {error}"))?;
            if milliseconds > 10_000 {
                return Err("sleep-ms maximum is 10000".into());
            }
            thread::sleep(Duration::from_millis(milliseconds));
            println!("slept {milliseconds}");
        }
        "emit-bytes" => {
            let count = one_argument(&mode, &arguments)?
                .parse::<usize>()
                .map_err(|error| format!("parse byte count: {error}"))?;
            if count > 2_000_000 {
                return Err("emit-bytes maximum is 2000000".into());
            }
            io::stdout()
                .write_all(&vec![b'x'; count])
                .map_err(|error| format!("write output: {error}"))?;
        }
        _ => return Err(format!("unknown mode {mode}")),
    }
    Ok(())
}

fn one_argument<'a>(mode: &str, arguments: &'a [String]) -> Result<&'a str, String> {
    if arguments.len() != 1 {
        return Err(format!("{mode} takes one argument"));
    }
    Ok(&arguments[0])
}
