use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;
use tode_core::LaunchTiming;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserScripts {
    pub preload: PathBuf,
    pub main_script: PathBuf,
}

const PRELOAD_SOURCE: &str = r#""use strict";
(() => {
  const { ipcRenderer } = require("electron");
  const deliver = (message) => ipcRenderer.send("tode:message", message);
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

pub fn write_browser_scripts(data_dir: &Path, css_file: &Path) -> io::Result<BrowserScripts> {
    fs::create_dir_all(data_dir)?;
    let preload = data_dir.join("browser-preload.js");
    let main_script = data_dir.join("browser-main.js");
    let timing_file = timing_path(css_file);
    let encoded_timing_file =
        serde_json::to_string(&timing_file.to_string_lossy().as_ref()).map_err(io::Error::other)?;
    let main_source = [
        r#""use strict";
(() => {
  const fs = require("node:fs");
  const { ipcMain } = require("electron");
  const timingFile = "#,
        &encoded_timing_file,
        r#";
  ipcMain.on("tode:message", (_event, message) => {
    if (!message || message.type !== "timing" || !message.page) return;
    try {
      fs.writeFileSync(timingFile, JSON.stringify(message.page));
    } catch {}
  });
})();
"#,
    ]
    .concat();
    write_if_changed(&preload, PRELOAD_SOURCE.as_bytes())?;
    write_if_changed(&main_script, main_source.as_bytes())?;
    Ok(BrowserScripts {
        preload,
        main_script,
    })
}

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
    fn writes_top_frame_timing_bridge_with_escaped_path() {
        let root = TempDir::new().unwrap();
        let css = root.path().join("quoted \"name\"/inject.css");
        let scripts = write_browser_scripts(&root.path().join("data"), &css).unwrap();
        let preload = fs::read_to_string(scripts.preload).unwrap();
        let main = fs::read_to_string(scripts.main_script).unwrap();
        assert!(preload.contains("window !== window.top"));
        assert!(preload.contains("code/didStartWorkbench"));
        assert!(preload.contains("type: \"timing\""));
        assert!(main.contains("ipcMain.on(\"tode:message\""));
        assert!(main.contains(r#"quoted \"name\"/inject.css.timing.json"#));
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
