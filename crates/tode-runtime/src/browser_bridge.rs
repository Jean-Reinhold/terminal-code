use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tempfile::NamedTempFile;
use tode_core::{GeneratedTheme, LaunchTiming, ParsedReplies, Rgb, generate_theme, with_fallbacks};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserScripts {
    pub preload: PathBuf,
    pub main_script: PathBuf,
}
#[derive(Debug, Deserialize)]
struct RawColors {
    background: Option<Rgb>,
    foreground: Option<Rgb>,
    #[serde(default)]
    ansi: Vec<Option<Rgb>>,
}

pub fn theme_from_raw(source: &str) -> Result<GeneratedTheme, String> {
    let raw: RawColors = serde_json::from_str(source).map_err(|error| error.to_string())?;
    let background = raw
        .background
        .ok_or_else(|| "terminal theme has no background".to_owned())?;
    let foreground = raw
        .foreground
        .ok_or_else(|| "terminal theme has no foreground".to_owned())?;
    let parsed = ParsedReplies {
        background: Some(background),
        foreground: Some(foreground),
        ansi: std::array::from_fn(|index| raw.ansi.get(index).copied().flatten()),
    };
    Ok(generate_theme(&with_fallbacks(Some(&parsed))))
}

const PRELOAD_SOURCE: &str = r#""use strict";
(() => {
  const { ipcRenderer } = require("electron");
  const deliver = (message) => ipcRenderer.send("tode:message", message);
  if (globalThis.terminalBrowser && typeof terminalBrowser.onTheme === "function") {
    terminalBrowser.onTheme((colors) => deliver({ type: "theme", colors }));
  }
  if (window !== window.top) return;
  const send = () => {
    try {
      const nav = performance.getEntriesByType("navigation")[0];
      const marks = {};
      for (const mark of performance.getEntriesByType("mark")) {
        if (mark.name.startsWith("code/")) marks[mark.name] = Math.round(mark.startTime);
      }
      if (Object.keys(marks).length === 0) return;
      deliver({
        type: "timing",
        page: {
          at: Date.now(),
          origin: Math.round(performance.timeOrigin),
          responseEnd: Math.round(nav?.responseEnd ?? 0),
          domInteractive: Math.round(nav?.domInteractive ?? 0),
          loadEnd: Math.round(nav?.loadEventEnd ?? 0),
          marks,
        },
      });
    } catch {}
  };
  let done = false;
  const settle = () => {
    if (done) return;
    done = true;
    setTimeout(send, 50);
  };
  const poll = setInterval(() => {
    if (performance.getEntriesByName("code/didStartWorkbench").length) {
      clearInterval(poll);
      settle();
    }
  }, 25);
  setTimeout(() => {
    clearInterval(poll);
    settle();
  }, 30000);
})();
"#;

pub fn write_browser_scripts(
    data_dir: &Path,
    css_file: &Path,
    theme_helper: &Path,
    socket_dir: &Path,
) -> io::Result<BrowserScripts> {
    fs::create_dir_all(data_dir)?;
    let preload = data_dir.join("browser-preload.js");
    let main_script = data_dir.join("browser-main.js");
    let context = serde_json::to_string(&serde_json::json!({
        "timingFile": timing_path(css_file),
        "themeHelper": theme_helper,
        "socketDir": socket_dir,
    }))
    .map_err(io::Error::other)?;
    let main_source = [
        "\"use strict\";\n((ctx) => {\n",
        MAIN_SOURCE,
        "\n})(",
        &context,
        ");\n",
    ]
    .concat();
    write_if_changed(&preload, PRELOAD_SOURCE.as_bytes())?;
    write_if_changed(&main_script, main_source.as_bytes())?;
    Ok(BrowserScripts {
        preload,
        main_script,
    })
}

const MAIN_SOURCE: &str = r#"
const fs = require("node:fs");
const net = require("node:net");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const { ipcMain } = require("electron");
const NL = String.fromCharCode(10);

function remove(socket) {
  try { fs.rmSync(socket, { force: true }); } catch {}
}

function send(socket, request) {
  const connection = net.createConnection(socket);
  let buffer = "";
  connection.on("connect", () => connection.end(JSON.stringify(request) + NL));
  connection.on("data", chunk => {
    buffer += chunk.toString("utf8");
    const newline = buffer.indexOf(NL);
    if (newline < 0) return;
    try {
      const reply = JSON.parse(buffer.slice(0, newline));
      if (!reply || reply.ok !== true) remove(socket);
    } catch { remove(socket); }
    connection.destroy();
  });
  connection.on("error", error => {
    if (error && (error.code === "ENOENT" || error.code === "ECONNREFUSED")) remove(socket);
  });
}

ipcMain.on("tode:message", (_event, message) => {
  if (message && message.type === "timing" && message.page) {
    try { fs.writeFileSync(ctx.timingFile, JSON.stringify(message.page)); } catch {}
    return;
  }
  if (!message || message.type !== "theme" || !message.colors) return;
  let theme;
  try {
    theme = JSON.parse(execFileSync(ctx.themeHelper, [], {
      input: JSON.stringify(message.colors),
      encoding: "utf8",
      maxBuffer: 4194304,
    }));
  } catch { return; }
  let names;
  try { names = fs.readdirSync(ctx.socketDir); } catch { return; }
  for (const name of names) {
    if (!name.endsWith(".sock")) continue;
    send(path.join(ctx.socketDir, name), {
      files: [], folders: [], add: false, wait: false, theme,
    });
  }
});
"#;

pub fn write_launch_timing(css_file: &Path, timing: &LaunchTiming) -> io::Result<PathBuf> {
    let path = launch_path(css_file);
    let contents = serde_json::to_vec(timing).map_err(io::Error::other)?;
    write_atomic(&path, &contents)?;
    Ok(path)
}

fn timing_path(css_file: &Path) -> PathBuf {
    PathBuf::from(format!("{}.timing.json", css_file.display()))
}

fn launch_path(css_file: &Path) -> PathBuf {
    PathBuf::from(format!("{}.launch.json", css_file.display()))
}

fn write_if_changed(path: &Path, contents: &[u8]) -> io::Result<()> {
    if fs::read(path).is_ok_and(|current| current == contents) {
        return Ok(());
    }
    write_atomic(path, contents)
}

fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("generated browser file has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(contents)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn writes_top_frame_timing_and_theme_bridge_with_escaped_paths() {
        let root = TempDir::new().unwrap();
        let css = root.path().join("quoted \"name\"/inject.css");
        let helper = root.path().join("helper \"theme\"");
        let sockets = root.path().join("socket \"dir\"");
        let scripts =
            write_browser_scripts(&root.path().join("data"), &css, &helper, &sockets).unwrap();
        let preload = fs::read_to_string(scripts.preload).unwrap();
        let main = fs::read_to_string(scripts.main_script).unwrap();
        assert!(preload.contains("window !== window.top"));
        assert!(preload.contains("code/didStartWorkbench"));
        assert!(preload.contains("type: \"timing\""));
        assert!(preload.contains("terminalBrowser.onTheme"));
        assert!(main.contains("ipcMain.on(\"tode:message\""));
        assert!(main.contains(r#"quoted \"name\"/inject.css.timing.json"#));
        assert!(main.contains(r#"helper \"theme\""#));
        assert!(main.contains(r#"socket \"dir\""#));
        assert!(main.contains("execFileSync(ctx.themeHelper"));
    }

    #[test]
    fn raw_terminal_colors_use_rust_theme_generation_and_fallbacks() {
        let theme = theme_from_raw(
            r#"{"background":[1,2,3],"foreground":[240,241,242],"ansi":[null,[9,8,7]]}"#,
        )
        .unwrap();
        assert_eq!(theme.colors["editor.background"], "#010203");
        assert_eq!(theme.colors["terminal.ansiRed"], "#090807");
    }

    #[test]
    fn raw_terminal_colors_require_background_and_foreground() {
        assert!(theme_from_raw(r#"{"foreground":[1,2,3]}"#).is_err());
        assert!(theme_from_raw("not json").is_err());
    }

    #[test]
    fn writes_launch_record_next_to_css() {
        let root = TempDir::new().unwrap();
        let css = root.path().join("inject.css");
        let path = write_launch_timing(
            &css,
            &LaunchTiming {
                spawned_at: 123,
                stages: vec![("runtime".into(), 10), ("profile".into(), 20)],
            },
        )
        .unwrap();
        let written: LaunchTiming = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(written.spawned_at, 123);
        assert_eq!(written.stages[1], ("profile".into(), 20));
    }
}
