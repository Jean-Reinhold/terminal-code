use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tode_core::{OpenRequest, hint_bindings, quit_chord, quit_when};

use crate::{ProfilePaths, write_if_changed};

pub const BRIDGE_ID: &str = "tode.tode-bridge";
pub const BRIDGE_VERSION: &str = "1.5.1";

pub fn bridge_directory(paths: &ProfilePaths) -> PathBuf {
    paths
        .extensions
        .join(format!("{BRIDGE_ID}-{BRIDGE_VERSION}"))
}

pub fn startup_open_file(paths: &ProfilePaths) -> PathBuf {
    paths.data.join("startup-open.json")
}

pub fn request_startup_open(
    paths: &ProfilePaths,
    request: &OpenRequest,
    now_ms: u128,
) -> std::io::Result<PathBuf> {
    let mut value = serde_json::to_value(request).expect("open request serializes");
    value["at"] = json!(now_ms);
    let mut bytes = serde_json::to_vec(&value).expect("startup request serializes");
    bytes.push(b'\n');
    let file = startup_open_file(paths);
    write_if_changed(&file, &bytes)?;
    Ok(file)
}

pub fn install_bridge(paths: &ProfilePaths) -> std::io::Result<bool> {
    let directory = bridge_directory(paths);
    let manifest = manifest();
    let mut manifest_bytes =
        serde_json::to_vec_pretty(&manifest).expect("bridge manifest serializes");
    manifest_bytes.push(b'\n');
    let wrote_manifest = write_if_changed(&directory.join("package.json"), &manifest_bytes)?;
    let source = bridge_source(paths);
    let wrote_source = write_if_changed(&directory.join("extension.js"), source.as_bytes())?;
    register_bridge(paths, &directory)?;
    Ok(wrote_manifest || wrote_source)
}

fn manifest() -> Value {
    let is_macos = cfg!(target_os = "macos");
    let quit = quit_chord(is_macos);
    let mut keybindings = vec![json!({
        "command": "tode.confirmQuit",
        "key": quit,
        "when": quit_when(is_macos),
    })];
    if quit == "ctrl+q"
        && let Some(hint) = hint_bindings(is_macos)
            .into_iter()
            .find(|binding| binding.command == "tode.quitHint")
    {
        keybindings.push(json!({
            "command": hint.command,
            "key": hint.key,
            "when": hint.when,
        }));
    }
    json!({
        "name": "tode-bridge",
        "displayName": "terminal-code",
        "publisher": "tode",
        "version": BRIDGE_VERSION,
        "engines": {"vscode": "^1.80.0"},
        "main": "./extension.js",
        "activationEvents": ["*"],
        "contributes": {
            "commands": [{
                "command": "tode.quit",
                "title": "Quit",
                "category": "terminal-code"
            }],
            "keybindings": keybindings
        }
    })
}

fn register_bridge(paths: &ProfilePaths, directory: &Path) -> std::io::Result<()> {
    let registry = paths.extensions.join("extensions.json");
    let Ok(source) = fs::read(&registry) else {
        return Ok(());
    };
    let Ok(mut entries) = serde_json::from_slice::<Vec<Value>>(&source) else {
        return Ok(());
    };
    entries
        .retain(|entry| entry.pointer("/identifier/id").and_then(Value::as_str) != Some(BRIDGE_ID));
    entries.push(json!({
        "identifier": {"id": BRIDGE_ID},
        "version": BRIDGE_VERSION,
        "relativeLocation": directory.file_name().and_then(|name| name.to_str()),
        "location": {"$mid": 1, "path": directory, "scheme": "file"},
        "metadata": {
            "isApplicationScoped": false,
            "isMachineScoped": false,
            "installedTimestamp": 0
        }
    }));
    let mut bytes = serde_json::to_vec_pretty(&entries).expect("bridge registry serializes");
    bytes.push(b'\n');
    write_if_changed(&registry, &bytes)?;
    Ok(())
}

fn bridge_source(paths: &ProfilePaths) -> String {
    let context = serde_json::to_string(&json!({
        "liveThemeFile": paths.data.join("live-theme.json"),
        "quitHint": "Use the terminal pane's quit command to close terminal-code.",
        "startupOpenFile": startup_open_file(paths),
        "socketDir": paths.state.join("ipc"),
    }))
    .expect("bridge context serializes");
    [
        "\"use strict\";\n(function(ctx){\n",
        BRIDGE_BODY,
        "\n})(",
        &context,
        ");\n",
    ]
    .concat()
}

const BRIDGE_BODY: &str = r#"
const fs = require("fs");
const net = require("net");
const path = require("path");
const vscode = require("vscode");
const NL = String.fromCharCode(10);
const VIEW_COMMANDS = { scm: "workbench.view.scm" };

function focusView(view) {
  const command = VIEW_COMMANDS[view];
  if (command) void vscode.commands.executeCommand(command);
}

function applyThemeDocument(theme) {
  if (!theme || typeof theme !== "object") return;
  const cfg = vscode.workspace.getConfiguration();
  const target = vscode.ConfigurationTarget.Global;
  if (theme.colors) cfg.update("workbench.colorCustomizations", theme.colors, target);
  if (theme.tokenColors) {
    cfg.update("editor.tokenColorCustomizations", { textMateRules: theme.tokenColors }, target);
  }
}

function applyLiveTheme() {
  try { applyThemeDocument(JSON.parse(fs.readFileSync(ctx.liveThemeFile, "utf8"))); } catch {}
}

function persistLiveTheme(theme) {
  try {
    fs.mkdirSync(path.dirname(ctx.liveThemeFile), { recursive: true });
    fs.writeFileSync(ctx.liveThemeFile + ".tmp", JSON.stringify(theme) + NL);
    fs.renameSync(ctx.liveThemeFile + ".tmp", ctx.liveThemeFile);
  } catch {}
}

function watchLiveTheme() {
  applyLiveTheme();
  const dir = path.dirname(ctx.liveThemeFile);
  const name = path.basename(ctx.liveThemeFile);
  let timer = null;
  let watcher = null;
  try {
    fs.mkdirSync(dir, { recursive: true });
    watcher = fs.watch(dir, { persistent: false }, (_event, filename) => {
      if (filename && filename !== name) return;
      if (timer) clearTimeout(timer);
      timer = setTimeout(applyLiveTheme, 30);
    });
  } catch {}
  return () => {
    if (timer) clearTimeout(timer);
    if (watcher) watcher.close();
  };
}

function socketPath() {
  fs.mkdirSync(ctx.socketDir, { recursive: true });
  return path.join(ctx.socketDir, `w${process.pid}-${Date.now()}.sock`);
}

function workspaceUri(target) {
  const folders = vscode.workspace.workspaceFolders;
  if (folders && folders.length > 0) return folders[0].uri.with({ path: target });
  if (vscode.env.remoteAuthority) {
    return vscode.Uri.from({ scheme: "vscode-remote", authority: vscode.env.remoteAuthority, path: target });
  }
  return vscode.Uri.file(target);
}

function sameUri(a, b) { return a.toString() === b.toString(); }
function alreadyOpen(uri) {
  return (vscode.workspace.workspaceFolders || []).some(folder => sameUri(folder.uri, uri));
}

function untilClosed(uris) {
  const waiting = uris.slice();
  const anyStillOpen = () => {
    const open = {};
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        const input = tab.input;
        if (input && input.uri) open[input.uri.toString()] = true;
        if (input && input.modified) open[input.modified.toString()] = true;
      }
    }
    return waiting.some(uri => open[uri]);
  };
  if (!anyStillOpen()) return Promise.resolve();
  return new Promise(resolve => {
    const subscription = vscode.window.tabGroups.onDidChangeTabs(() => {
      if (anyStillOpen()) return;
      subscription.dispose();
      resolve();
    });
  });
}

async function open(request, acknowledge) {
  if (request.theme) {
    applyThemeDocument(request.theme);
    persistLiveTheme(request.theme);
    return;
  }
  if (request.view) focusView(request.view);
  if (request.diff && request.diff.length === 2) {
    await vscode.commands.executeCommand(
      "vscode.diff",
      vscode.Uri.file(request.diff[0]),
      vscode.Uri.file(request.diff[1])
    );
  }
  const opened = [];
  for (const file of request.files || []) {
    const uri = fs.existsSync(file.path)
      ? vscode.Uri.file(file.path)
      : vscode.Uri.file(file.path).with({ scheme: "untitled" });
    const document = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(document, { preview: false });
    opened.push(document.uri.toString());
    if (file.line) {
      const line = Math.max(0, file.line - 1);
      const column = Math.max(0, (file.column || 1) - 1);
      const at = new vscode.Position(line, column);
      editor.selection = new vscode.Selection(at, at);
      editor.revealRange(new vscode.Range(at, at), vscode.TextEditorRevealType.InCenter);
    }
  }
  const wanted = (request.folders || []).map(workspaceUri).filter(uri => !alreadyOpen(uri));
  if (wanted.length === 0) {
    if (request.wait && opened.length > 0) await untilClosed(opened);
    acknowledge();
    return;
  }
  acknowledge();
  for (const uri of wanted) {
    if (request.add) {
      const at = (vscode.workspace.workspaceFolders || []).length;
      vscode.workspace.updateWorkspaceFolders(at, 0, { uri });
    } else {
      await vscode.commands.executeCommand("vscode.openFolder", uri, { forceNewWindow: false });
    }
  }
}

function applyStartupOpen() {
  let parsed;
  try { parsed = JSON.parse(fs.readFileSync(ctx.startupOpenFile, "utf8")); } catch { return; }
  try { fs.rmSync(ctx.startupOpenFile, { force: true }); } catch {}
  if (!parsed || Date.now() - (parsed.at || 0) > 120000) return;
  void open({ files: [], folders: [], add: false, ...parsed, wait: false }, () => {});
}

function quitTode() {
  void vscode.env.openExternal(vscode.Uri.parse("terminal-browser://quit"));
}

function activate(context) {
  context.subscriptions.push(vscode.commands.registerCommand("tode.quit", quitTode));
  let confirmShowing = false;
  context.subscriptions.push(vscode.commands.registerCommand("tode.confirmQuit", () => {
    if (confirmShowing) return;
    confirmShowing = true;
    vscode.window.showErrorMessage(
      "Do you want to quit terminal-code?", { modal: true }, "Quit"
    ).then(picked => {
      confirmShowing = false;
      if (picked === "Quit") quitTode();
    }, () => { confirmShowing = false; });
  }));
  let hintShowing = false;
  context.subscriptions.push(vscode.commands.registerCommand("tode.quitHint", () => {
    if (hintShowing) return;
    hintShowing = true;
    const done = () => { hintShowing = false; };
    vscode.window.showErrorMessage(ctx.quitHint, { modal: true }).then(done, done);
  }));
  applyStartupOpen();
  context.subscriptions.push({ dispose: watchLiveTheme() });
  const sock = socketPath();
  const server = net.createServer(connection => {
    let buffer = "";
    connection.on("data", chunk => {
      buffer += chunk.toString("utf8");
      if (buffer.length > 1048576) {
        connection.end(JSON.stringify({ ok: false, error: "request too large" }) + NL);
        buffer = "";
        return;
      }
      const newline = buffer.indexOf(NL);
      if (newline < 0) return;
      const line = buffer.slice(0, newline);
      buffer = "";
      let request;
      try { request = JSON.parse(line); }
      catch {
        connection.end(JSON.stringify({ ok: false, error: "bad request" }) + NL);
        return;
      }
      let answered = false;
      const acknowledge = () => {
        if (answered) return;
        answered = true;
        connection.end(JSON.stringify({ ok: true }) + NL);
      };
      Promise.resolve(open(request, acknowledge)).then(acknowledge, error => {
        if (answered) return;
        answered = true;
        connection.end(JSON.stringify({ ok: false, error: String(error) }) + NL);
      });
    });
    connection.on("error", () => {});
  });
  server.on("error", () => {});
  server.listen(sock, () => context.environmentVariableCollection.replace("TODE_IPC", sock));
  context.subscriptions.push({ dispose: () => {
    try { server.close(); } catch {}
    try { fs.rmSync(sock, { force: true }); } catch {}
  }});
}

module.exports.activate = activate;
module.exports.deactivate = () => {};
"#;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use tempfile::TempDir;

    #[test]
    fn installs_registers_and_reuses_dependency_free_bridge() {
        let root = TempDir::new().unwrap();
        let paths = ProfilePaths::from_environment(root.path(), &BTreeMap::new());
        fs::create_dir_all(&paths.extensions).unwrap();
        fs::write(
            paths.extensions.join("extensions.json"),
            r#"[{"identifier":{"id":"tode.tode-bridge"},"version":"old"},{"identifier":{"id":"other"}}]"#,
        )
        .unwrap();
        assert!(install_bridge(&paths).unwrap());
        assert!(!install_bridge(&paths).unwrap());
        let directory = bridge_directory(&paths);
        let manifest: Value =
            serde_json::from_slice(&fs::read(directory.join("package.json")).unwrap()).unwrap();
        assert_eq!(manifest["activationEvents"][0], "*");
        let source = fs::read_to_string(directory.join("extension.js")).unwrap();
        assert!(source.contains("module.exports.activate = activate"));
        assert!(source.contains("environmentVariableCollection.replace(\"TODE_IPC\""));
        let registry: Vec<Value> =
            serde_json::from_slice(&fs::read(paths.extensions.join("extensions.json")).unwrap())
                .unwrap();
        assert_eq!(
            registry
                .iter()
                .filter(|entry| entry["identifier"]["id"] == BRIDGE_ID)
                .count(),
            1
        );
        assert!(
            registry
                .iter()
                .any(|entry| entry["identifier"]["id"] == "other")
        );
    }

    #[test]
    fn startup_marker_carries_open_request_and_freshness_time() {
        let root = TempDir::new().unwrap();
        let paths = ProfilePaths::from_environment(root.path(), &BTreeMap::new());
        let request = OpenRequest {
            files: vec![tode_core::OpenFile {
                path: "/tmp/main.rs".into(),
                line: Some(12),
                column: Some(4),
            }],
            folders: Vec::new(),
            add: false,
            wait: Some(false),
            diff: Some(vec!["/tmp/a".into(), "/tmp/b".into()]),
            view: Some("scm".into()),
            theme: None,
        };
        let file = request_startup_open(&paths, &request, 123).unwrap();
        let value: Value = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
        assert_eq!(value["at"], 123);
        assert_eq!(value["files"][0]["line"], 12);
        assert_eq!(value["diff"][1], "/tmp/b");
        assert_eq!(value["view"], "scm");
    }
}
