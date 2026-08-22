use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tode_profile::{ProfilePaths, THEME_EXTENSION_ID, managed_setting_keys, seeded_setting_keys};
use tode_runtime::{current_server, now_unix_ms, origin};

pub struct SkillContext<'a> {
    pub home: &'a Path,
    pub environment: &'a BTreeMap<OsString, OsString>,
    pub paths: &'a ProfilePaths,
    pub terminal_browser_version: &'a str,
    pub code_server_version: &'a str,
}

pub async fn skill_text(context: &SkillContext<'_>) -> String {
    let paths = context.paths;
    let version = read_trimmed(&paths.install_root.join("VERSION"));
    let channel = read_trimmed(&paths.install_root.join("CHANNEL"));
    let install = version.as_ref().map_or_else(
        || {
            "a working tree, not a release (no VERSION file — upgrade and uninstall refuse it)"
                .to_owned()
        },
        |version| {
            format!(
                "release {version} on channel {}",
                channel.as_deref().unwrap_or("unknown")
            )
        },
    );
    let bin_home = environment_path(context.environment, "XDG_BIN_HOME")
        .unwrap_or_else(|| context.home.join(".local/bin"));
    let shim = bin_home.join("tode");
    let state_file = paths.state.join("server.json");
    let server = current_server(&state_file, Duration::from_millis(200)).await;
    let daemon = server.as_ref().map_or_else(
        || "not running — the next `tode` starts it".to_owned(),
        |server| {
            let age_minutes = now_unix_ms().saturating_sub(server.started_at) / 60_000;
            format!(
                "up {age_minutes}m — code-server pid {} on 127.0.0.1:{}, injector pid {} on 127.0.0.1:{}; windows load {} (always the injector, never code-server directly)",
                server.pid,
                server.port,
                server.injector_pid,
                server.injector_port,
                origin(server)
            )
        },
    );
    let code_server =
        environment_path(context.environment, "TODE_CODE_SERVER").unwrap_or_else(|| {
            paths
                .data
                .join("code-server")
                .join(context.code_server_version)
                .join("bin/code-server")
        });
    let code_server_state = if code_server.is_file() {
        code_server.display().to_string()
    } else {
        format!(
            "not fetched yet — the first open puts it under {}",
            paths.data.join("code-server").display()
        )
    };
    let ipc_dir = paths.state.join("ipc");
    let sockets: Vec<_> = list_dir(&ipc_dir)
        .into_iter()
        .filter(|entry| entry.ends_with(".sock"))
        .collect();
    let extensions: Vec<_> = list_dir(&paths.extensions)
        .into_iter()
        .filter(|entry| !entry.starts_with('.') && entry != "extensions.json")
        .collect();
    let terminals = detected_terminals(context.home, context.environment);
    let in_window = context
        .environment
        .get(OsStr::new("TODE_IPC"))
        .and_then(|value| value.to_str());
    let runtime_sources = runtime_sources(context);
    let vscode = paths.data.join("vscode");
    let live_theme = paths.data.join("live-theme.json");
    let css = paths.data.join("inject.css");
    let browser_app_data = paths.data.join("browser/chromium");
    let managed = managed_setting_keys().join(", ");
    let seeded = seeded_setting_keys().join(", ");

    let mut output = String::new();
    writeln!(output, "---").unwrap();
    writeln!(output, "name: tode").unwrap();
    writeln!(output, "description: Working knowledge of this machine's tode install (the terminal code editor). Where its code-server profile, extensions, settings, themes, generated browser bridge, logs and daemon state live, which files are safe to edit and which are regenerated, and how to reach running windows. Regenerate with `tode --skill`; every path and state value is resolved live.").unwrap();
    writeln!(output, "---\n").unwrap();
    writeln!(output, "# tode\n").unwrap();
    writeln!(output, "tode is a code editor that runs in the terminal: one warm code-server serves the VS Code workbench, a Rust injecting proxy sits in front of it, and terminal-browser draws each window as a terminal pane. Everything below was resolved when `tode --skill` ran; re-run it rather than trusting a copy.\n").unwrap();

    writeln!(output, "## State right now\n").unwrap();
    writeln!(
        output,
        "- install root: {} — {install}",
        paths.install_root.display()
    )
    .unwrap();
    writeln!(
        output,
        "- shim: {} {}",
        shim.display(),
        if shim.is_file() {
            "(present)"
        } else {
            "(absent)"
        }
    )
    .unwrap();
    writeln!(
        output,
        "- terminal-browser pin {}: {runtime_sources}",
        context.terminal_browser_version
    )
    .unwrap();
    writeln!(
        output,
        "- code-server {}: {code_server_state}",
        context.code_server_version
    )
    .unwrap();
    writeln!(output, "- daemon: {daemon}").unwrap();
    writeln!(
        output,
        "- open windows: {} socket(s) in {}{}",
        sockets.len(),
        ipc_dir.display(),
        if sockets.is_empty() {
            String::new()
        } else {
            format!(" — {}", sockets.join(", "))
        }
    )
    .unwrap();
    writeln!(
        output,
        "- this shell {}",
        in_window.map_or_else(
            || "is not inside a tode window (no TODE_IPC)".to_owned(),
            |socket| format!("is inside a tode window (TODE_IPC={socket})")
        )
    )
    .unwrap();
    writeln!(
        output,
        "- terminal detected for shortcut overrides: {}",
        if terminals.is_empty() {
            "none (ghostty and kitty are supported)".to_owned()
        } else {
            terminals.join("; ")
        }
    )
    .unwrap();
    writeln!(
        output,
        "- installed extensions ({}): {}\n",
        extensions.len(),
        if extensions.is_empty() {
            "none".to_owned()
        } else {
            extensions.join(", ")
        }
    )
    .unwrap();

    writeln!(
        output,
        "## The editor profile — settings, keybindings, snippets, extensions\n"
    )
    .unwrap();
    writeln!(output, "There is exactly one profile under {}. Flags such as --profile and --user-data-dir are deliberately swallowed; every window and extension operation uses this profile.\n", vscode.display()).unwrap();
    writeln!(
        output,
        "- {} — live settings. tode rewrites these managed keys on every open: {managed}.",
        paths.user.join("settings.json").display()
    )
    .unwrap();
    writeln!(
        output,
        "  These keys are seeded once and user edits win: {seeded}. Every other key is untouched."
    )
    .unwrap();
    writeln!(output, "- {} — user keybindings. Import deduplicates entries; interactive shortcut convergence is not installed until `tode --shortcut-setup` completes successfully.", paths.user.join("keybindings.json").display()).unwrap();
    writeln!(
        output,
        "- {} and tasks.json — plain VS Code files copied by import.",
        paths.user.join("snippets").display()
    )
    .unwrap();
    writeln!(output, "- {} — the extension tree plus extensions.json. Manage it with `tode --install-extension`, `--uninstall-extension`, and `--list-extensions`.", paths.extensions.display()).unwrap();
    writeln!(output, "- {THEME_EXTENSION_ID}-<fingerprint> is generated by tode and rewritten on theme installation; do not hand-edit it.\n").unwrap();

    writeln!(output, "## Theme and generated browser files\n").unwrap();
    writeln!(
        output,
        "- `tode --theme` rebuilds the terminal-derived theme."
    )
    .unwrap();
    writeln!(
        output,
        "- {} — full generated live theme; regenerated on theme install.",
        live_theme.display()
    )
    .unwrap();
    writeln!(
        output,
        "- {} — CSS injected into every workbench page; regenerated on open.",
        css.display()
    )
    .unwrap();
    writeln!(
        output,
        "- {} and {} — generated Electron preload/main scripts; regenerated on open.",
        paths.data.join("browser-preload.js").display(),
        paths.data.join("browser-main.js").display()
    )
    .unwrap();
    writeln!(
        output,
        "- {}.timing.json and .launch.json feed `tode --timing`.\n",
        css.display()
    )
    .unwrap();

    writeln!(output, "## Terminal shortcut overrides\n").unwrap();
    writeln!(output, "`tode --shortcut-setup` owns only terminal-code include files after the interactive conflict resolver succeeds. Ghostty uses `config-file = ?tode/keybinds.ghostty`; kitty uses `include tode/keybinds.kitty.conf`. Re-run the resolver instead of editing generated files.\n").unwrap();

    writeln!(output, "## Reaching a running window\n").unwrap();
    writeln!(output, "A window bridge exports its Unix socket as TODE_IPC. The CLI sends one JSON line containing files, folders, add/wait, diff, view, or theme fields. Inside a window, `tode <file>` opens there, `tode -r <folder>` reuses it, and `--review` focuses source control. New-window goto/diff/review still require the bridge startup path.\n").unwrap();

    writeln!(output, "## Daemon and processes\n").unwrap();
    writeln!(
        output,
        "- {} — code-server and injector PIDs/ports; treat as read-only and use `tode --shutdown`.",
        state_file.display()
    )
    .unwrap();
    writeln!(
        output,
        "- {} — daemon launcher log.",
        paths.logs.join("tode-daemon.log").display()
    )
    .unwrap();
    writeln!(
        output,
        "- {} — combined managed code-server output.",
        paths.logs.join("code-server.log").display()
    )
    .unwrap();
    writeln!(output, "- code-server binds 127.0.0.1 with --auth none; workbench URLs always use the Rust injector port.\n").unwrap();

    writeln!(output, "## Homes on disk\n").unwrap();
    writeln!(output, "- data {} — profile, theme, generated scripts, code-server and terminal-browser runtime trees", paths.data.display()).unwrap();
    writeln!(
        output,
        "- state {} — daemon state, logs and IPC sockets",
        paths.state.display()
    )
    .unwrap();
    writeln!(
        output,
        "- cache {} — tode and browser caches",
        paths.cache.display()
    )
    .unwrap();
    writeln!(
        output,
        "- terminal-browser isolated homes: data {}, state {}, cache {}, chromium {}\n",
        paths.browser_data.display(),
        paths.browser_state.display(),
        paths.browser_cache.display(),
        browser_app_data.display()
    )
    .unwrap();

    writeln!(output, "## Environment variables\n").unwrap();
    writeln!(output, "TODE_IPC selects a running window; TODE_INSTALL_ROOT, TODE_CODE_SERVER, TODE_TERMINAL_BROWSER_BIN and TODE_RELEASE_ORIGIN override install/runtime sources. TODE_BROWSER_DATA/_STATE/_CACHE/_RUN/_APPDATA move browser homes. XDG_DATA_HOME, XDG_STATE_HOME, XDG_CACHE_HOME and XDG_BIN_HOME move tode homes and the shim.\n").unwrap();

    writeln!(output, "## Commands\n").unwrap();
    writeln!(output, "Open with `tode [path...]` and -g/-d/-a/-r/-n/-w, --split/--size, --review, or extension flags. First-position commands are --shortcut-setup, --import, --theme, --timing, --skill, --upgrade, --shutdown and --uninstall.").unwrap();
    output
}

fn runtime_sources(context: &SkillContext<'_>) -> String {
    let mut sources = Vec::new();
    if let Some(path) = environment_path(context.environment, "TODE_TERMINAL_BROWSER_BIN") {
        sources.push(format!(
            "override via TODE_TERMINAL_BROWSER_BIN at {}",
            path.display()
        ));
    }
    let vendored = context.paths.install_root.join("vendor/terminal-browser");
    if vendored.is_dir() {
        sources.push(format!("vendored in {}", vendored.display()));
    }
    let fetched = context
        .paths
        .runtime
        .join("terminal-browser")
        .join(context.terminal_browser_version);
    if fetched.is_dir() {
        sources.push(format!("fetched at {}", fetched.display()));
    }
    if sources.is_empty() {
        "not on disk yet — the next open downloads it".to_owned()
    } else {
        sources.join("; ")
    }
}

fn detected_terminals(home: &Path, environment: &BTreeMap<OsString, OsString>) -> Vec<String> {
    let mut terminals = Vec::new();
    if environment
        .get(OsStr::new("TERM_PROGRAM"))
        .is_some_and(|value| value == "ghostty")
        || environment.contains_key(OsStr::new("GHOSTTY_RESOURCES_DIR"))
    {
        terminals.push(format!(
            "ghostty, config in {}",
            ghostty_config_dir(home, environment).display()
        ));
    }
    if environment
        .get(OsStr::new("TERM"))
        .is_some_and(|value| value == "xterm-kitty")
        || environment.contains_key(OsStr::new("KITTY_WINDOW_ID"))
        || environment.contains_key(OsStr::new("KITTY_PID"))
    {
        terminals.push(format!(
            "kitty, config in {}",
            kitty_config_dir(home, environment).display()
        ));
    }
    terminals
}

fn ghostty_config_dir(home: &Path, environment: &BTreeMap<OsString, OsString>) -> PathBuf {
    let config_home = absolute_environment_path(environment, "XDG_CONFIG_HOME")
        .unwrap_or_else(|| home.join(".config"));
    let candidates = [
        config_home.join("ghostty"),
        home.join("Library/Application Support/com.mitchellh.ghostty"),
    ];
    candidates
        .iter()
        .find(|directory| directory.join("config").is_file())
        .cloned()
        .unwrap_or_else(|| candidates[usize::from(cfg!(target_os = "macos"))].clone())
}

fn kitty_config_dir(home: &Path, environment: &BTreeMap<OsString, OsString>) -> PathBuf {
    let config_home = absolute_environment_path(environment, "XDG_CONFIG_HOME")
        .unwrap_or_else(|| home.join(".config"));
    let mut candidates = Vec::new();
    if let Some(directory) = absolute_environment_path(environment, "KITTY_CONFIG_DIRECTORY") {
        candidates.push(directory);
    }
    candidates.push(config_home.join("kitty"));
    candidates.push(home.join("Library/Preferences/kitty"));
    candidates
        .iter()
        .find(|directory| directory.join("kitty.conf").is_file())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

fn absolute_environment_path(
    environment: &BTreeMap<OsString, OsString>,
    name: &str,
) -> Option<PathBuf> {
    environment_path(environment, name).filter(|path| path.is_absolute())
}

fn environment_path(environment: &BTreeMap<OsString, OsString>, name: &str) -> Option<PathBuf> {
    environment.get(OsStr::new(name)).map(PathBuf::from)
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn list_dir(path: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut names: Vec<_> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}
