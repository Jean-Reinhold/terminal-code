use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use tode_core::{
    HELP, LaunchTiming, OpenFile, OpenRequest, PageTiming, ReleaseManifest, build_for,
    current_target_triple, format_timing, installed_receipt, installed_version,
    latest_manifest_path, parse_goto, resolve_target, send_to_extension, with_fallbacks,
    workbench_url,
};
use tode_profile::{
    FONT_FAMILY, ProfilePaths, UninstallConfig, find_editors, install_settings, install_theme,
    run_import, uninstall, write_if_changed,
};
use tode_runtime::{
    BrowserHomes, RuntimeRoots, ServerState, UpgradeOutcome, apply_build, current_server,
    injected_css, resolve_runtime, stop_server, write_browser_scripts, write_launch_timing,
};

const TERMINAL_BROWSER_VERSION: &str = "v0.5.8";
const CODE_SERVER_VERSION: &str = "4.132.0";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct OpenOptions {
    paths: Vec<String>,
    goto: bool,
    add: bool,
    diff: bool,
    new_window: bool,
    reuse: bool,
    wait: bool,
    split: Option<String>,
    size: Option<String>,
    review: bool,
    timing: bool,
    warnings: Vec<String>,
    install_extensions: Vec<String>,
    uninstall_extensions: Vec<String>,
    list_extensions: bool,
    show_versions: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum CliCommand {
    Help,
    Version,
    Shutdown,
    Timing,
    Import(Option<String>),
    Theme(Option<String>),
    Uninstall(bool),
    Upgrade {
        check: bool,
        version: Option<String>,
    },
    Open(OpenOptions),
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
        CliCommand::Import(name) => import_editor(name.as_deref(), &home, &environment, &paths),
        CliCommand::Theme(file) => set_theme(file.as_deref(), &paths),
        CliCommand::Timing => timing_command(&paths),
        CliCommand::Uninstall(yes) => uninstall_command(yes, &home, &environment, &paths),
        CliCommand::Upgrade { check, version } => {
            upgrade_command(check, version.as_deref(), &environment, &paths).await
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
        CliCommand::Open(options) => {
            for warning in &options.warnings {
                eprintln!("{warning}");
            }
            open(options, &paths, &environment).await
        }
    }
}

async fn open(
    options: OpenOptions,
    paths: &ProfilePaths,
    environment: &BTreeMap<OsString, OsString>,
) -> Result<u8, String> {
    if !options.install_extensions.is_empty()
        || !options.uninstall_extensions.is_empty()
        || options.list_extensions
    {
        return manage_extensions(&options, paths, environment);
    }
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let files = open_files(&options, &cwd)?;
    let wanted: Vec<_> = if options.goto || options.diff {
        Vec::new()
    } else {
        options
            .paths
            .iter()
            .map(|path| resolve_target(Some(path), &cwd))
            .collect()
    };
    let folders: Vec<String> = wanted
        .iter()
        .filter_map(|target| target.folder.as_ref())
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let diff = if options.diff {
        Some(
            options
                .paths
                .iter()
                .map(|path| absolute(&cwd, path).to_string_lossy().into_owned())
                .collect(),
        )
    } else {
        None
    };
    if !options.new_window
        && let Some(socket) = running_window(environment)
    {
        let here = options.add || options.reuse;
        let send_folders = if here { folders.clone() } else { Vec::new() };
        let opens_pane = !folders.is_empty() && !here;
        if !opens_pane
            && (!files.is_empty() || !send_folders.is_empty() || diff.is_some() || options.review)
        {
            let request = OpenRequest {
                files,
                folders: send_folders,
                add: options.add,
                wait: Some(options.wait),
                diff,
                view: options.review.then(|| "scm".into()),
                theme: None,
            };
            let timeout = (!options.wait).then_some(Duration::from_secs(4));
            tokio::task::spawn_blocking(move || send_to_extension(&socket, &request, timeout))
                .await
                .map_err(|error| format!("IPC task failed: {error}"))?
                .map_err(|error| format!("could not reach the tode window: {error}"))?;
            return Ok(0);
        }
    }
    if options.goto || options.diff || options.review || options.add || options.reuse {
        return Err(
            "goto/add/diff/review/reuse requires an existing Rust bridge window for now".into(),
        );
    }
    if options.paths.len() > 1 {
        return Err("multiple new-window targets require the Rust bridge for now".into());
    }
    let target = resolve_target(options.paths.first().map(String::as_str), &cwd);
    let started = std::time::Instant::now();
    let mut stages = Vec::new();
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
    stages.push((
        "runtime".to_owned(),
        i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
    ));

    let palette = with_fallbacks(None);
    install_settings(paths).map_err(|error| format!("install settings: {error}"))?;
    install_theme(paths, &palette).map_err(|error| format!("install theme: {error}"))?;
    let css = injected_css(&tode_core::hex(palette.background), FONT_FAMILY);
    let css_file = paths.data.join("inject.css");
    write_if_changed(&css_file, css.as_bytes()).map_err(|error| format!("install CSS: {error}"))?;
    stages.push((
        "profile".to_owned(),
        i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
    ));

    let state_file = paths.state.join("server.json");
    let state = match current_server(&state_file, Duration::from_millis(400)).await {
        Some(state) => state,
        None => start_daemon(paths, environment, &css_file, &state_file)?,
    };
    stages.push((
        "code-server".to_owned(),
        i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
    ));
    let url = workbench_url(&tode_runtime::origin(&state), &target)
        .map_err(|error| format!("build workbench URL: {error}"))?;
    let scripts = write_browser_scripts(&paths.data, &css_file)
        .map_err(|error| format!("install browser bridge: {error}"))?;
    let spawned_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;
    write_launch_timing(
        &css_file,
        &LaunchTiming {
            spawned_at,
            stages: stages.clone(),
        },
    )
    .map_err(|error| format!("record launch timing: {error}"))?;
    if options.timing {
        for (label, milliseconds) in &stages {
            eprintln!("  {label:<12} {milliseconds}ms");
        }
    }
    let mut browser_arguments = vec![
        "open".to_owned(),
        url,
        "--app-mode".to_owned(),
        format!("--preload={}", scripts.preload.display()),
        format!("--main-script={}", scripts.main_script.display()),
    ];
    if let Some(split) = options.split {
        browser_arguments.extend(["--split".into(), split]);
    }
    if let Some(size) = options.size {
        browser_arguments.extend(["--size".into(), size]);
    }
    let status = Command::new(&runtime.bin)
        .args(browser_arguments)
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
    let code_server = code_server_path(paths, environment)?;
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

fn code_server_path(
    paths: &ProfilePaths,
    environment: &BTreeMap<OsString, OsString>,
) -> Result<PathBuf, String> {
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
    if code_server.is_file() {
        Ok(code_server)
    } else {
        Err(format!(
            "code-server {CODE_SERVER_VERSION} not found at {}",
            code_server.display()
        ))
    }
}

fn manage_extensions(
    options: &OpenOptions,
    paths: &ProfilePaths,
    environment: &BTreeMap<OsString, OsString>,
) -> Result<u8, String> {
    let binary = code_server_path(paths, environment)?;
    let run = |arguments: &[String]| -> Result<i32, String> {
        let status = Command::new(&binary)
            .args(arguments)
            .args([
                OsString::from("--extensions-dir"),
                paths.extensions.clone().into_os_string(),
                OsString::from("--user-data-dir"),
                paths.data.join("vscode/user-data").into_os_string(),
            ])
            .status()
            .map_err(|error| format!("run code-server extension command: {error}"))?;
        Ok(status.code().unwrap_or(1))
    };
    if options.list_extensions {
        let mut arguments = vec!["--list-extensions".to_owned()];
        if options.show_versions {
            arguments.push("--show-versions".into());
        }
        return Ok(run(&arguments)?.clamp(0, 255) as u8);
    }
    for extension in &options.uninstall_extensions {
        let code = run(&["--uninstall-extension".into(), extension.clone()])?;
        if code != 0 {
            return Ok(code.clamp(0, 255) as u8);
        }
    }
    for extension in &options.install_extensions {
        let code = run(&["--install-extension".into(), extension.clone()])?;
        if code != 0 {
            return Ok(code.clamp(0, 255) as u8);
        }
    }
    if !options.install_extensions.is_empty() {
        println!("open tode again to pick it up");
    }
    Ok(0)
}

fn import_editor(
    name: Option<&str>,
    home: &Path,
    environment: &BTreeMap<OsString, OsString>,
    paths: &ProfilePaths,
) -> Result<u8, String> {
    let xdg = environment
        .get(&OsString::from("XDG_CONFIG_HOME"))
        .map(PathBuf::from);
    let editors = find_editors(home, xdg.as_deref(), cfg!(target_os = "macos"));
    let editor = match name {
        Some(name) => editors
            .iter()
            .find(|editor| editor.name.eq_ignore_ascii_case(name)),
        None => editors.first(),
    }
    .ok_or_else(|| match name {
        Some(name) => format!("no compatible editor named {name}"),
        None => "no compatible editor found".into(),
    })?;
    let report = run_import(editor, paths);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("import report serializes")
    );
    Ok(0)
}

fn set_theme(file: Option<&str>, paths: &ProfilePaths) -> Result<u8, String> {
    if let Some(file) = file {
        return Err(format!(
            "theme file import is not yet available in Rust: {file}"
        ));
    }
    let installed =
        install_theme(paths, &with_fallbacks(None)).map_err(|error| error.to_string())?;
    println!(
        "theme {} {}",
        installed.fingerprint,
        if installed.changed {
            "written"
        } else {
            "already current"
        }
    );
    Ok(0)
}

fn timing_command(paths: &ProfilePaths) -> Result<u8, String> {
    let css = paths.data.join("inject.css");
    let page_file = PathBuf::from(format!("{}.timing.json", css.display()));
    let launch_file = PathBuf::from(format!("{}.launch.json", css.display()));
    let page: PageTiming = match fs::read(&page_file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    {
        Some(page) => page,
        None => {
            println!("no page timing recorded yet, open tode once");
            return Ok(0);
        }
    };
    let launch: Option<LaunchTiming> = fs::read(&launch_file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;
    print!("{}", format_timing(&page, launch.as_ref(), now));
    Ok(0)
}

fn uninstall_command(
    yes: bool,
    home: &Path,
    environment: &BTreeMap<OsString, OsString>,
    paths: &ProfilePaths,
) -> Result<u8, String> {
    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err("pass --yes to uninstall without a prompt".into());
        }
        print!("Uninstall terminal-code? [y/N] ");
        std::io::stdout()
            .flush()
            .map_err(|error| error.to_string())?;
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .map_err(|error| error.to_string())?;
        if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            return Ok(0);
        }
    }
    stop_server(&paths.state.join("server.json"));
    let data_home = paths.data.parent().unwrap_or(&paths.data);
    let config_home = environment
        .get(&OsString::from("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    let bin_home = environment
        .get(&OsString::from("XDG_BIN_HOME"))
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".local/bin"));
    let font = if cfg!(target_os = "macos") {
        home.join("Library/Fonts/JetBrainsMono-Regular.ttf")
    } else {
        data_home.join("fonts/JetBrainsMono-Regular.ttf")
    };
    uninstall(&UninstallConfig {
        paths: paths.clone(),
        install_roots: vec![paths.install_root.clone(), home.join(".local/lib/tode")],
        shim: bin_home.join("tode"),
        font,
        bundled_font: paths
            .install_root
            .join("assets/fonts/JetBrainsMono-Regular.ttf"),
        ghostty_config: config_home.join("ghostty"),
        kitty_config: config_home.join("kitty"),
    })
    .map_err(|error| format!("uninstall: {error}"))?;
    println!("done");
    Ok(0)
}

async fn upgrade_command(
    check: bool,
    version: Option<&str>,
    environment: &BTreeMap<OsString, OsString>,
    paths: &ProfilePaths,
) -> Result<u8, String> {
    let origin = environment
        .get(&OsString::from("TODE_RELEASE_ORIGIN"))
        .and_then(|value| value.to_str())
        .unwrap_or("https://tode.sh/install")
        .trim_end_matches('/');
    let installed = installed_receipt(&paths.install_root);
    let url = match version {
        Some(version) => format!("{origin}/v/{version}/manifest.json"),
        None => {
            let channel = installed
                .as_ref()
                .map(|receipt| receipt.channel.as_str())
                .unwrap_or("stable");
            format!("{origin}{}", latest_manifest_path(channel))
        }
    };
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("could not read {url}: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("could not read {url} ({})", response.status()));
    }
    let manifest: ReleaseManifest = response
        .json()
        .await
        .map_err(|error| format!("invalid release manifest from {url}: {error}"))?;
    let build = build_for(&manifest, &current_target_triple())?;
    match apply_build(&client, &build, &paths.install_root, check)
        .await
        .map_err(|error| error.to_string())?
    {
        UpgradeOutcome::Current { version, channel } => {
            println!("tode {version} is the newest on {channel}");
        }
        UpgradeOutcome::Available { from: _, build } => {
            println!(
                "tode {} is available (you have {})",
                build.version,
                installed
                    .as_ref()
                    .map(|receipt| receipt.version.as_str())
                    .unwrap_or("unknown")
            );
        }
        UpgradeOutcome::Upgraded { from, build } => {
            stop_server(&paths.state.join("server.json"));
            println!("tode {from} -> {}", build.version);
        }
    }
    Ok(0)
}

fn parse_command(arguments: Vec<String>) -> Result<CliCommand, String> {
    match arguments.as_slice() {
        [flag] if matches!(flag.as_str(), "--help" | "-h") => return Ok(CliCommand::Help),
        [flag] if matches!(flag.as_str(), "--version" | "-v") => return Ok(CliCommand::Version),
        [flag] if flag == "--shutdown" => return Ok(CliCommand::Shutdown),
        [flag] if flag == "--timing" => return Ok(CliCommand::Timing),
        [flag, rest @ ..] if flag == "--upgrade" => {
            let mut check = false;
            let mut version = None;
            let mut index = 0;
            while index < rest.len() {
                match rest[index].as_str() {
                    "--check" => check = true,
                    "--version" => {
                        version = Some(
                            rest.get(index + 1)
                                .ok_or_else(|| "--version needs a value".to_owned())?
                                .clone(),
                        );
                        index += 1;
                    }
                    option => return Err(format!("unknown upgrade option {option}")),
                }
                index += 1;
            }
            return Ok(CliCommand::Upgrade { check, version });
        }
        [flag, rest @ ..] if flag == "--import" => {
            if rest.len() > 1 {
                return Err("--import takes at most one editor name".into());
            }
            return Ok(CliCommand::Import(rest.first().cloned()));
        }
        [flag, rest @ ..] if flag == "--theme" => {
            if rest.len() > 1 {
                return Err("--theme takes at most one file".into());
            }
            return Ok(CliCommand::Theme(rest.first().cloned()));
        }
        [flag, rest @ ..] if flag == "--uninstall" => {
            if rest
                .iter()
                .all(|argument| matches!(argument.as_str(), "--yes" | "-y"))
            {
                return Ok(CliCommand::Uninstall(!rest.is_empty()));
            }
            return Err("--uninstall accepts only --yes or -y".into());
        }
        _ => {}
    }
    let mut options = OpenOptions::default();
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        match argument.as_str() {
            "-g" | "--goto" => options.goto = true,
            "-a" | "--add" => options.add = true,
            "-d" | "--diff" => options.diff = true,
            "-n" | "--new-window" => options.new_window = true,
            "-r" | "--reuse-window" => options.reuse = true,
            "-w" | "--wait" => options.wait = true,
            "--review" => options.review = true,
            "--timing" => options.timing = true,
            "--verbose"
            | "--disable-gpu"
            | "--disable-telemetry"
            | "--disable-updates"
            | "--no-sandbox"
            | "--skip-release-notes"
            | "--skip-welcome"
            | "--disable-workspace-trust" => {}
            "--log" | "--locale" | "--sync" | "--profile" | "--user-data-dir"
            | "--extensions-dir" => {
                if arguments.get(index + 1).is_none() {
                    return Err(format!("{argument} needs a value"));
                }
                index += 1;
            }
            "--disable-extensions" | "--disable-extension" => {
                options.warnings.push(format!(
                    "tode: ignoring {argument}, extensions are per code-server, not per window"
                ));
            }
            "--install-extension" | "--uninstall-extension" => {
                let value = arguments
                    .get(index + 1)
                    .ok_or_else(|| format!("{argument} needs a value"))?
                    .clone();
                if argument == "--install-extension" {
                    options.install_extensions.push(value);
                } else {
                    options.uninstall_extensions.push(value);
                }
                index += 1;
            }
            "--list-extensions" => options.list_extensions = true,
            "--show-versions" => options.show_versions = true,
            "--split" | "--size" => {
                let value = arguments
                    .get(index + 1)
                    .ok_or_else(|| format!("{argument} needs a value"))?
                    .clone();
                if argument == "--split" {
                    if !matches!(value.as_str(), "right" | "left" | "down" | "up") {
                        return Err(format!("invalid --split direction {value}"));
                    }
                    options.split = Some(value);
                } else {
                    let fraction = value
                        .parse::<f64>()
                        .map_err(|error| format!("invalid --size: {error}"))?;
                    if !(0.2..=0.95).contains(&fraction) {
                        return Err("--size must be between 0.2 and 0.95".into());
                    }
                    options.size = Some(value);
                }
                index += 1;
            }
            value if value.starts_with('-') => return Err(format!("unknown option {value}")),
            value => options.paths.push(value.to_owned()),
        }
        index += 1;
    }
    if options.size.is_some() && options.split.is_none() {
        return Err("--size only applies with --split".into());
    }
    if options.diff && options.paths.len() != 2 {
        return Err("--diff takes two files".into());
    }
    if options.show_versions && !options.list_extensions {
        return Err("--show-versions only applies with --list-extensions".into());
    }
    Ok(CliCommand::Open(options))
}

fn running_window(environment: &BTreeMap<OsString, OsString>) -> Option<PathBuf> {
    let path = environment
        .get(&OsString::from("TODE_IPC"))
        .map(PathBuf::from)?;
    fs::metadata(&path)
        .ok()
        .filter(|metadata| metadata.file_type().is_socket())
        .map(|_| path)
}

fn open_files(options: &OpenOptions, cwd: &Path) -> Result<Vec<OpenFile>, String> {
    if options.diff {
        return Ok(Vec::new());
    }
    if options.goto {
        return Ok(options
            .paths
            .iter()
            .map(|argument| {
                let mut file = parse_goto(argument, cwd);
                file.path = absolute(cwd, &file.path).to_string_lossy().into_owned();
                file
            })
            .collect());
    }
    Ok(options
        .paths
        .iter()
        .filter_map(|path| {
            let target = resolve_target(Some(path), cwd);
            target.file.map(|file| OpenFile {
                path: file.to_string_lossy().into_owned(),
                line: None,
                column: None,
            })
        })
        .collect())
}

fn absolute(cwd: &Path, value: &str) -> PathBuf {
    let target = resolve_target(Some(value), cwd);
    target
        .file
        .or(target.folder)
        .unwrap_or_else(|| cwd.to_path_buf())
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
    fn parses_identity_profile_shutdown_and_default_open() {
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
            parse_command(vec!["--timing".into()]).unwrap(),
            CliCommand::Timing
        );
        assert_eq!(
            parse_command(vec!["--import".into()]).unwrap(),
            CliCommand::Import(None)
        );
        assert_eq!(
            parse_command(vec!["--import".into(), "Code".into()]).unwrap(),
            CliCommand::Import(Some("Code".into()))
        );
        assert_eq!(
            parse_command(vec!["--theme".into()]).unwrap(),
            CliCommand::Theme(None)
        );
        assert_eq!(
            parse_command(vec!["--uninstall".into()]).unwrap(),
            CliCommand::Uninstall(false)
        );
        assert_eq!(
            parse_command(vec!["--uninstall".into(), "--yes".into()]).unwrap(),
            CliCommand::Uninstall(true)
        );
        assert_eq!(
            parse_command(vec![
                "--upgrade".into(),
                "--check".into(),
                "--version".into(),
                "v2".into(),
            ])
            .unwrap(),
            CliCommand::Upgrade {
                check: true,
                version: Some("v2".into()),
            }
        );
        assert_eq!(
            parse_command(Vec::new()).unwrap(),
            CliCommand::Open(OpenOptions::default())
        );
    }

    #[test]
    fn parses_goto_add_diff_wait_review_and_split_options() {
        let goto = parse_command(vec![
            "--goto".into(),
            "--wait".into(),
            "src/main.rs:12:4".into(),
        ])
        .unwrap();
        assert_eq!(
            goto,
            CliCommand::Open(OpenOptions {
                paths: vec!["src/main.rs:12:4".into()],
                goto: true,
                wait: true,
                ..OpenOptions::default()
            })
        );
        let split = parse_command(vec![
            "--new-window".into(),
            "--split".into(),
            "right".into(),
            "--size".into(),
            "0.4".into(),
            "--timing".into(),
            "folder".into(),
        ])
        .unwrap();
        assert_eq!(
            split,
            CliCommand::Open(OpenOptions {
                paths: vec!["folder".into()],
                new_window: true,
                split: Some("right".into()),
                size: Some("0.4".into()),
                timing: true,
                ..OpenOptions::default()
            })
        );
        assert!(matches!(
            parse_command(vec!["--diff".into(), "a".into(), "b".into()]).unwrap(),
            CliCommand::Open(OpenOptions { diff: true, .. })
        ));
    }

    #[test]
    fn consumes_ignored_flags_and_preserves_unsupported_warnings() {
        let command = parse_command(vec![
            "--verbose".into(),
            "--locale".into(),
            "en-US".into(),
            "--disable-extensions".into(),
            "folder".into(),
        ])
        .unwrap();
        let CliCommand::Open(options) = command else {
            panic!("expected open command");
        };
        assert_eq!(options.paths, ["folder"]);
        assert_eq!(
            options.warnings,
            ["tode: ignoring --disable-extensions, extensions are per code-server, not per window"]
        );
        assert!(parse_command(vec!["--locale".into()]).is_err());
    }

    #[test]
    fn parses_repeated_extension_operations_and_list_versions() {
        let command = parse_command(vec![
            "--uninstall-extension".into(),
            "old.one".into(),
            "--install-extension".into(),
            "new.one".into(),
            "--install-extension".into(),
            "new.two".into(),
        ])
        .unwrap();
        let CliCommand::Open(options) = command else {
            panic!("expected open command");
        };
        assert_eq!(options.uninstall_extensions, ["old.one"]);
        assert_eq!(options.install_extensions, ["new.one", "new.two"]);
        let list =
            parse_command(vec!["--list-extensions".into(), "--show-versions".into()]).unwrap();
        assert!(matches!(
            list,
            CliCommand::Open(OpenOptions {
                list_extensions: true,
                show_versions: true,
                ..
            })
        ));
        assert!(parse_command(vec!["--show-versions".into()]).is_err());
    }

    #[test]
    fn rejects_invalid_open_options() {
        assert!(parse_command(vec!["--diff".into()]).is_err());
        assert!(parse_command(vec!["--size".into(), "0.5".into()]).is_err());
        assert!(parse_command(vec!["--split".into(), "diagonal".into()]).is_err());
        assert!(
            parse_command(vec![
                "--size".into(),
                "1.5".into(),
                "--split".into(),
                "right".into()
            ])
            .is_err()
        );
        assert!(parse_command(vec!["--unknown".into()]).is_err());
    }
}
