use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use tode_core::{HELP, installed_version, resolve_target, with_fallbacks, workbench_url};
use tode_profile::{FONT_FAMILY, ProfilePaths, install_settings, install_theme, write_if_changed};
use tode_runtime::{
    BrowserHomes, RuntimeRoots, ServerState, current_server, injected_css, resolve_runtime,
    stop_server,
};

const TERMINAL_BROWSER_VERSION: &str = "v0.5.8";
const CODE_SERVER_VERSION: &str = "4.132.0";

#[derive(Debug, PartialEq, Eq)]
enum CliCommand {
    Help,
    Version,
    Shutdown,
    Open(Vec<String>),
}

#[tokio::main]
async fn main() -> ExitCode {
    match execute().await {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("tode: {error}");
            ExitCode::from(1)
        }
    }
}

async fn execute() -> Result<u8, String> {
    let command = parse_command(env::args().skip(1).collect())?;
    let environment: BTreeMap<OsString, OsString> = env::vars_os().collect();
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_owned())?;
    let paths = ProfilePaths::from_environment(&home, &environment);
    match command {
        CliCommand::Help => {
            print!("{HELP}");
            Ok(0)
        }
        CliCommand::Version => {
            println!("{}", installed_version(&paths.install_root));
            Ok(0)
        }
        CliCommand::Shutdown => {
            let stopped = stop_server(&paths.state.join("server.json"));
            println!(
                "{}",
                if stopped {
                    "tode stopped"
                } else {
                    "nothing was running"
                }
            );
            Ok(0)
        }
        CliCommand::Open(arguments) => open(arguments, &paths, &environment).await,
    }
}

async fn open(
    arguments: Vec<String>,
    paths: &ProfilePaths,
    environment: &BTreeMap<OsString, OsString>,
) -> Result<u8, String> {
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let target = resolve_target(arguments.first().map(String::as_str), &cwd);
    let palette = with_fallbacks(None);
    install_settings(paths).map_err(|error| format!("install settings: {error}"))?;
    install_theme(paths, &palette).map_err(|error| format!("install theme: {error}"))?;
    let css = injected_css(&tode_core::hex(palette.background), FONT_FAMILY);
    let css_file = paths.data.join("inject.css");
    write_if_changed(&css_file, css.as_bytes()).map_err(|error| format!("install CSS: {error}"))?;

    let state_file = paths.state.join("server.json");
    let state = match current_server(&state_file, Duration::from_millis(400)).await {
        Some(state) => state,
        None => start_daemon(paths, environment, &css_file, &state_file)?,
    };
    let url = workbench_url(&tode_runtime::origin(&state), &target)
        .map_err(|error| format!("build workbench URL: {error}"))?;
    let client = reqwest::Client::new();
    let roots = runtime_roots(paths);
    let override_binary = environment
        .get(&OsString::from("TODE_TERMINAL_BROWSER_BIN"))
        .map(PathBuf::from);
    let release_origin = environment
        .get(&OsString::from("TODE_RELEASE_ORIGIN"))
        .and_then(|value| value.to_str())
        .unwrap_or("https://terminal-browser.sh/install");
    let runtime = resolve_runtime(
        &client,
        &roots,
        TERMINAL_BROWSER_VERSION,
        TERMINAL_BROWSER_VERSION,
        override_binary.as_deref(),
        cfg!(target_os = "macos"),
        release_origin,
    )
    .await
    .map_err(|error| error.to_string())?;
    let status = Command::new(&runtime.bin)
        .args(["open", &url, "--app-mode"])
        .status()
        .map_err(|error| format!("could not start terminal-browser: {error}"))?;
    Ok(status.code().unwrap_or(1).clamp(0, 255) as u8)
}

fn start_daemon(
    paths: &ProfilePaths,
    environment: &BTreeMap<OsString, OsString>,
    css_file: &Path,
    state_file: &Path,
) -> Result<ServerState, String> {
    let code_server = environment
        .get(&OsString::from("TODE_CODE_SERVER"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            paths
                .data
                .join("code-server")
                .join(CODE_SERVER_VERSION)
                .join("bin/code-server")
        });
    if !code_server.is_file() {
        return Err(format!(
            "code-server {CODE_SERVER_VERSION} not found at {}",
            code_server.display()
        ));
    }
    let daemon = environment
        .get(&OsString::from("TODE_DAEMON"))
        .map(PathBuf::from)
        .unwrap_or_else(default_daemon_path);
    if !daemon.is_file() {
        return Err(format!("tode-daemon not found at {}", daemon.display()));
    }
    let reservation = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
        .map_err(|error| format!("reserve code-server port: {error}"))?;
    let port = reservation
        .local_addr()
        .map_err(|error| format!("read reserved port: {error}"))?
        .port();
    drop(reservation);
    fs::create_dir_all(&paths.logs)
        .map_err(|error| format!("create daemon log directory: {error}"))?;
    let daemon_log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.logs.join("tode-daemon.log"))
        .map_err(|error| format!("open daemon log: {error}"))?;
    let mut child = Command::new(daemon)
        .args([
            OsString::from("--code-server"),
            code_server.into_os_string(),
            OsString::from("--code-port"),
            OsString::from(port.to_string()),
            OsString::from("--user-data"),
            paths.data.join("vscode/user-data").into_os_string(),
            OsString::from("--extensions"),
            paths.extensions.clone().into_os_string(),
            OsString::from("--log"),
            paths.logs.join("code-server.log").into_os_string(),
            OsString::from("--css"),
            css_file.as_os_str().to_owned(),
            OsString::from("--state"),
            state_file.as_os_str().to_owned(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(daemon_log))
        .process_group(0)
        .spawn()
        .map_err(|error| format!("start tode-daemon: {error}"))?;
    let mut line = String::new();
    BufReader::new(
        child
            .stdout
            .take()
            .ok_or_else(|| "tode-daemon stdout unavailable".to_owned())?,
    )
    .read_line(&mut line)
    .map_err(|error| format!("read tode-daemon readiness: {error}"))?;
    if line.trim().is_empty() {
        return Err("tode-daemon exited before readiness".into());
    }
    serde_json::from_str(line.trim()).map_err(|error| format!("read daemon state: {error}"))
}

fn parse_command(arguments: Vec<String>) -> Result<CliCommand, String> {
    match arguments.as_slice() {
        [flag] if matches!(flag.as_str(), "--help" | "-h") => Ok(CliCommand::Help),
        [flag] if matches!(flag.as_str(), "--version" | "-v") => Ok(CliCommand::Version),
        [flag] if flag == "--shutdown" => Ok(CliCommand::Shutdown),
        values if values.iter().any(|value| value.starts_with('-')) => {
            let unknown = values
                .iter()
                .find(|value| value.starts_with('-'))
                .expect("checked above");
            Err(format!("unknown option {unknown}"))
        }
        values if values.len() <= 1 => Ok(CliCommand::Open(values.to_vec())),
        _ => Err("multiple targets are not implemented in the Rust CLI yet".into()),
    }
}

fn default_daemon_path() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("tode-daemon")))
        .unwrap_or_else(|| PathBuf::from("tode-daemon"))
}

fn runtime_roots(paths: &ProfilePaths) -> RuntimeRoots {
    let xdg_data = paths.data.parent().unwrap_or(&paths.data);
    RuntimeRoots {
        runtime: paths.runtime.clone(),
        vendored: paths.install_root.join("vendor/terminal-browser"),
        system_install: xdg_data.join("terminal-browser/app"),
        homes: BrowserHomes {
            data: paths.browser_data.clone(),
            state: paths.browser_state.clone(),
            cache: paths.browser_cache.clone(),
            app_data: paths.data.join("browser/chromium"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_identity_shutdown_and_single_open() {
        assert_eq!(
            parse_command(vec!["--help".into()]).unwrap(),
            CliCommand::Help
        );
        assert_eq!(
            parse_command(vec!["-v".into()]).unwrap(),
            CliCommand::Version
        );
        assert_eq!(
            parse_command(vec!["--shutdown".into()]).unwrap(),
            CliCommand::Shutdown
        );
        assert_eq!(
            parse_command(Vec::new()).unwrap(),
            CliCommand::Open(Vec::new())
        );
        assert_eq!(
            parse_command(vec!["folder".into()]).unwrap(),
            CliCommand::Open(vec!["folder".into()])
        );
        assert!(parse_command(vec!["--diff".into()]).is_err());
        assert!(parse_command(vec!["one".into(), "two".into()]).is_err());
    }
}
