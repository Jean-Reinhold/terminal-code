use std::io::{self, Read, Write};
use std::process::ExitCode;

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
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, &theme).map_err(|error| error.to_string())?;
    output.write_all(b"\n").map_err(|error| error.to_string())
}
