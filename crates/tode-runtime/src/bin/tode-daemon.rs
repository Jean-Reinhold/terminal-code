use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use tode_runtime::{CodeServerConfig, Daemon, DaemonConfig};

#[derive(Debug)]
struct Arguments {
    code_server: PathBuf,
    code_port: u16,
    user_data: PathBuf,
    extensions: PathBuf,
    log: PathBuf,
    css: PathBuf,
    font: Option<PathBuf>,
    state: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    let arguments = match parse_arguments(env::args().skip(1).collect()) {
        Ok(Some(arguments)) => arguments,
        Ok(None) => {
            print_help();
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("tode-daemon: {error}");
            return ExitCode::from(2);
        }
    };
    let daemon = match Daemon::start(DaemonConfig {
        code_server: CodeServerConfig {
            binary: arguments.code_server,
            port: arguments.code_port,
            user_data: arguments.user_data,
            extensions: arguments.extensions,
            log_file: arguments.log,
            readiness_deadline: Duration::from_secs(30),
        },
        injector_listen: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        css_file: arguments.css,
        font_file: arguments.font,
        injector_hold: Duration::from_secs(20),
        state_file: arguments.state,
    })
    .await
    {
        Ok(daemon) => daemon,
        Err(error) => {
            eprintln!("tode-daemon: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string(&daemon.state) {
        Ok(state) => println!("{state}"),
        Err(error) => {
            eprintln!("tode-daemon: serialize state: {error}");
            let _ = daemon.shutdown().await;
            return ExitCode::from(1);
        }
    }
    wait_for_shutdown().await;
    match daemon.shutdown().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tode-daemon: {error}");
            ExitCode::from(1)
        }
    }
}

async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn parse_arguments(arguments: Vec<String>) -> Result<Option<Arguments>, String> {
    if arguments
        .iter()
        .any(|argument| matches!(argument.as_str(), "--help" | "-h"))
    {
        return Ok(None);
    }
    let mut code_server = None;
    let mut code_port = None;
    let mut user_data = None;
    let mut extensions = None;
    let mut log = None;
    let mut css = None;
    let mut font = None;
    let mut state = None;
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--code-server" => code_server = Some(value.into()),
            "--code-port" => {
                code_port = Some(
                    value
                        .parse::<u16>()
                        .map_err(|error| format!("invalid --code-port: {error}"))?,
                )
            }
            "--user-data" => user_data = Some(value.into()),
            "--extensions" => extensions = Some(value.into()),
            "--log" => log = Some(value.into()),
            "--css" => css = Some(value.into()),
            "--font" => font = Some(value.into()),
            "--state" => state = Some(value.into()),
            _ => return Err(format!("unknown option {flag}")),
        }
        index += 2;
    }
    Ok(Some(Arguments {
        code_server: required(code_server, "--code-server")?,
        code_port: required(code_port, "--code-port")?,
        user_data: required(user_data, "--user-data")?,
        extensions: required(extensions, "--extensions")?,
        log: required(log, "--log")?,
        css: required(css, "--css")?,
        font,
        state: required(state, "--state")?,
    }))
}

fn required<T>(value: Option<T>, flag: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("missing {flag}"))
}

fn print_help() {
    println!(
        "Usage: tode-daemon --code-server <path> --code-port <port> --user-data <dir> --extensions <dir> --log <file> --css <file> [--font <file>] --state <file>"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_complete_pairs_and_rejects_unknown_options() {
        assert!(parse_arguments(vec!["--code-server".into()]).is_err());
        assert!(parse_arguments(vec!["--unknown".into(), "x".into()]).is_err());
        assert!(parse_arguments(vec!["--help".into()]).unwrap().is_none());
    }
}
