use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use tode_core::{OpenRequest, parse_goto, resolve_target, send_to_extension};

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
        "ipc-json" => {
            let request = one_argument(&mode, &arguments)?;
            serde_json::from_str::<serde_json::Value>(request)
                .map_err(|error| format!("parse request JSON: {error}"))?;
            let socket = env::var("TODE_IPC").map_err(|_| "TODE_IPC is not set".to_owned())?;
            let mut stream = UnixStream::connect(&socket)
                .map_err(|error| format!("connect {socket}: {error}"))?;
            stream
                .write_all(request.as_bytes())
                .and_then(|()| stream.write_all(b"\n"))
                .map_err(|error| format!("write request: {error}"))?;
            let mut response = String::new();
            BufReader::new(stream)
                .read_line(&mut response)
                .map_err(|error| format!("read response: {error}"))?;
            print!("{response}");
        }
        "ipc-open" => ipc_open(&mode, &arguments, Some(Duration::from_secs(4)))?,
        "ipc-open-wait" => ipc_open(&mode, &arguments, None)?,
        "ipc-open-timeout" => {
            if arguments.len() != 2 {
                return Err("ipc-open-timeout takes a request and milliseconds".into());
            }
            let timeout = arguments[1]
                .parse::<u64>()
                .map_err(|error| format!("parse timeout milliseconds: {error}"))?;
            ipc_open(&mode, &arguments[..1], Some(Duration::from_millis(timeout)))?;
        }
        _ => return Err(format!("unknown mode {mode}")),
    }
    Ok(())
}

fn ipc_open(mode: &str, arguments: &[String], timeout: Option<Duration>) -> Result<(), String> {
    let request: OpenRequest = serde_json::from_str(one_argument(mode, arguments)?)
        .map_err(|error| format!("parse request JSON: {error}"))?;
    let socket = env::var("TODE_IPC").map_err(|_| "TODE_IPC is not set".to_owned())?;
    send_to_extension(std::path::Path::new(&socket), &request, timeout)
        .map_err(|error| error.to_string())
}

fn one_argument<'a>(mode: &str, arguments: &'a [String]) -> Result<&'a str, String> {
    if arguments.len() != 1 {
        return Err(format!("{mode} takes one argument"));
    }
    Ok(&arguments[0])
}
