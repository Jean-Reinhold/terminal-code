use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

const INPUT_LIMIT: u64 = 64 * 1024;

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tode-theme-bridge: {error}");
            ExitCode::FAILURE
        }
    }
}

fn execute() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let socket_dir = match arguments.as_slice() {
        [] => None,
        [flag, path] if flag == "--socket-dir" => Some(PathBuf::from(path)),
        _ => return Err("usage: tode-theme-bridge [--socket-dir <path>]".into()),
    };
    let mut input = Vec::new();
    io::stdin()
        .take(INPUT_LIMIT + 1)
        .read_to_end(&mut input)
        .map_err(|error| error.to_string())?;
    if input.len() as u64 > INPUT_LIMIT {
        return Err(format!("input exceeds {INPUT_LIMIT} bytes"));
    }
    let source = std::str::from_utf8(&input).map_err(|error| error.to_string())?;
    let theme = tode_runtime::browser_bridge::theme_from_raw(source)?;
    if let Some(socket_dir) = socket_dir {
        tode_runtime::browser_bridge::fanout_theme(&socket_dir, &theme, Duration::from_secs(4))
            .map_err(|error| error.to_string())?;
    }
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, &theme).map_err(|error| error.to_string())?;
    output.write_all(b"\n").map_err(|error| error.to_string())
}
