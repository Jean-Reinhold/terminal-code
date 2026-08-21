use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tode_core::{parse_jsonc, set_key};

use crate::{ProfilePaths, apply_settings, write_if_changed};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor {
    pub name: String,
    pub user_dir: PathBuf,
    pub extensions_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorContents {
    pub settings: bool,
    pub keybindings: bool,
    pub snippets: usize,
    pub tasks: bool,
    pub extensions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportReport {
    pub settings: Option<SettingsReport>,
    pub keybindings: Option<usize>,
    pub snippets: Vec<String>,
    pub tasks: bool,
    pub extensions: ExtensionsReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsReport {
    pub imported: usize,
    pub kept_by_tode: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionsReport {
    pub copied: Vec<String>,
    pub skipped: Vec<SkippedExtension>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedExtension {
    pub id: String,
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Binding {
    key: Option<String>,
    command: Option<String>,
    when: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtensionEntry {
    identifier: ExtensionIdentifier,
    version: String,
    #[serde(rename = "relativeLocation")]
    relative_location: Option<String>,
    location: Option<ExtensionLocation>,
    metadata: Option<BTreeMap<String, Value>>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtensionIdentifier {
    id: String,
    uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtensionLocation {
    path: Option<String>,
    scheme: Option<String>,
    #[serde(rename = "$mid")]
    mid: Option<u64>,
}

pub fn find_editors(home: &Path, xdg_config_home: Option<&Path>, is_macos: bool) -> Vec<Editor> {
    let support = if is_macos {
        home.join("Library/Application Support")
    } else {
        xdg_config_home
            .filter(|path| path.is_absolute())
            .map(Path::to_owned)
            .unwrap_or_else(|| home.join(".config"))
    };
    let Ok(entries) = fs::read_dir(&support) else {
        return Vec::new();
    };
    let mut editors = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let user_dir = support.join(&name).join("User");
        let state = user_dir.join("globalStorage/state.vscdb");
        if !state.is_file() {
            continue;
        }
        let last_used = fs::metadata(&state)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_millis());
        editors.push((
            last_used,
            Editor {
                extensions_dir: find_extensions_dir(home, &name),
                name,
                user_dir,
            },
        ));
    }
    editors.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    editors.into_iter().map(|(_, editor)| editor).collect()
}

pub fn describe(editor: &Editor) -> EditorContents {
    let snippets = fs::read_dir(editor.user_dir.join("snippets"))
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension, "json" | "code-snippets"))
        })
        .count();
    EditorContents {
        settings: editor.user_dir.join("settings.json").is_file(),
        keybindings: editor.user_dir.join("keybindings.json").is_file(),
        snippets,
        tasks: editor.user_dir.join("tasks.json").is_file(),
        extensions: editor
            .extensions_dir
            .as_ref()
            .map_or(0, |directory| count_extensions(directory)),
    }
}

pub fn summarise(contents: &EditorContents) -> String {
    let mut parts = Vec::new();
    if contents.extensions > 0 {
        parts.push(format!("{} extensions", contents.extensions));
    }
    if contents.settings {
        parts.push("settings".into());
    }
    if contents.keybindings {
        parts.push("keybindings".into());
    }
    if contents.snippets > 0 {
        parts.push(format!("{} snippet files", contents.snippets));
    }
    if contents.tasks {
        parts.push("tasks".into());
    }
    if parts.is_empty() {
        "nothing to import".into()
    } else {
        parts.join(", ")
    }
}

pub fn run_import_with_progress(
    editor: &Editor,
    paths: &ProfilePaths,
    progress: &mut dyn FnMut(usize, usize, &str),
) -> ImportReport {
    ImportReport {
        extensions: import_extensions(editor, paths, progress),
        settings: import_settings(editor, paths),
        keybindings: import_keybindings(editor, paths),
        snippets: import_snippets(editor, paths),
        tasks: import_tasks(editor, paths),
    }
}

pub fn run_import(editor: &Editor, paths: &ProfilePaths) -> ImportReport {
    run_import_with_progress(editor, paths, &mut |_, _, _| {})
}

fn find_extensions_dir(home: &Path, name: &str) -> Option<PathBuf> {
    let slug = name
        .to_lowercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();
    let first = name.split_whitespace().next().unwrap_or(name);
    let first_slug = first
        .to_lowercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();
    let known = match name {
        "Code" => Some(".vscode"),
        "Code - Insiders" => Some(".vscode-insiders"),
        "code-oss-dev" | "VSCodium" => Some(".vscode-oss"),
        _ => None,
    };
    let candidates = [
        known.map(str::to_owned),
        Some(format!(".{slug}")),
        Some(format!(".{first_slug}")),
    ];
    candidates.into_iter().flatten().find_map(|candidate| {
        let directory = home.join(candidate).join("extensions");
        directory
            .join("extensions.json")
            .is_file()
            .then_some(directory)
    })
}

fn count_extensions(directory: &Path) -> usize {
    fs::read(directory.join("extensions.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<Value>>(&bytes).ok())
        .map_or(0, |entries| entries.len())
}

fn import_settings(editor: &Editor, paths: &ProfilePaths) -> Option<SettingsReport> {
    let source = fs::read_to_string(editor.user_dir.join("settings.json")).ok()?;
    let theirs: serde_json::Map<String, Value> = parse_jsonc(&source)?;
    let target = paths.user.join("settings.json");
    let mut current = fs::read_to_string(&target).unwrap_or_else(|_| "{}".into());
    for (key, value) in &theirs {
        current = set_key(&current, key, value);
    }
    let managed = managed_keys();
    let kept_by_tode = theirs
        .keys()
        .filter(|key| managed.contains(&key.as_str()))
        .cloned()
        .collect();
    let output = apply_settings(&current);
    write_if_changed(&target, output.as_bytes()).ok()?;
    Some(SettingsReport {
        imported: theirs.len(),
        kept_by_tode,
    })
}

fn import_keybindings(editor: &Editor, paths: &ProfilePaths) -> Option<usize> {
    let source = fs::read_to_string(editor.user_dir.join("keybindings.json")).ok()?;
    let theirs: Vec<Binding> = parse_jsonc(&source)?;
    let target = paths.user.join("keybindings.json");
    let existing: Vec<Binding> = fs::read_to_string(&target)
        .ok()
        .and_then(|source| parse_jsonc(&source))
        .unwrap_or_default();
    let mut merged = existing;
    let mut added = 0;
    for binding in theirs {
        if !merged.contains(&binding) {
            merged.push(binding);
            added += 1;
        }
    }
    let output = format!(
        "// \n{}\n",
        serde_json::to_string_pretty(&merged).expect("bindings serialize")
    );
    write_if_changed(&target, output.as_bytes()).ok()?;
    Some(added)
}

fn import_snippets(editor: &Editor, paths: &ProfilePaths) -> Vec<String> {
    let source = editor.user_dir.join("snippets");
    let Ok(entries) = fs::read_dir(source) else {
        return Vec::new();
    };
    let target = paths.user.join("snippets");
    let _ = fs::create_dir_all(&target);
    let mut copied = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name();
        if fs::copy(&path, target.join(&name)).is_ok() {
            copied.push(name.to_string_lossy().into_owned());
        }
    }
    copied.sort();
    copied
}

fn import_tasks(editor: &Editor, paths: &ProfilePaths) -> bool {
    let source = editor.user_dir.join("tasks.json");
    source.is_file()
        && fs::create_dir_all(&paths.user).is_ok()
        && fs::copy(source, paths.user.join("tasks.json")).is_ok()
}

fn import_extensions(
    editor: &Editor,
    paths: &ProfilePaths,
    progress: &mut dyn FnMut(usize, usize, &str),
) -> ExtensionsReport {
    let mut report = ExtensionsReport {
        copied: Vec::new(),
        skipped: Vec::new(),
    };
    let Some(source_root) = &editor.extensions_dir else {
        return report;
    };
    let listed: Vec<ExtensionEntry> = match fs::read(source_root.join("extensions.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    {
        Some(listed) => listed,
        None => {
            report.skipped.push(SkippedExtension {
                id: "extensions.json".into(),
                why: "could not be read".into(),
            });
            return report;
        }
    };
    let _ = fs::create_dir_all(&paths.extensions);
    let target_registry = paths.extensions.join("extensions.json");
    let existing: Vec<ExtensionEntry> = fs::read(&target_registry)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let mut kept: BTreeMap<_, _> = existing
        .into_iter()
        .map(|entry| (entry.identifier.id.clone(), entry))
        .collect();

    let total = listed.len();
    for (done, mut entry) in listed.into_iter().enumerate() {
        progress(done + 1, total, &entry.identifier.id);
        let folder = entry
            .relative_location
            .clone()
            .or_else(|| {
                entry
                    .location
                    .as_ref()
                    .and_then(|location| location.path.as_ref())
                    .and_then(|path| Path::new(path).file_name())
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_default();
        if !safe_relative(&folder) {
            report.skipped.push(SkippedExtension {
                id: entry.identifier.id,
                why: "its folder is unsafe".into(),
            });
            continue;
        }
        let source = source_root.join(&folder);
        if !source.exists() {
            report.skipped.push(SkippedExtension {
                id: entry.identifier.id,
                why: "its folder is missing".into(),
            });
            continue;
        }
        let target = paths.extensions.join(&folder);
        if copy_tree(&source, &target).is_err() {
            report.skipped.push(SkippedExtension {
                id: entry.identifier.id,
                why: "could not be copied".into(),
            });
            continue;
        }
        entry.relative_location = Some(folder);
        entry.location = Some(ExtensionLocation {
            path: Some(target.to_string_lossy().into_owned()),
            scheme: Some("file".into()),
            mid: Some(1),
        });
        report.copied.push(entry.identifier.id.clone());
        kept.insert(entry.identifier.id.clone(), entry);
    }
    let registry: Vec<_> = kept.into_values().collect();
    let output = format!(
        "{}\n",
        serde_json::to_string_pretty(&registry).expect("extension registry serializes")
    );
    let _ = write_if_changed(&target_registry, output.as_bytes());
    report
}

fn copy_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    if target.exists() {
        fs::remove_dir_all(target)?;
    }
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .expect("walk stays under source");
        let destination = target.join(relative);
        if entry.file_type().is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "extension tree contains a symlink",
            ));
        }
        if entry.file_type().is_dir() {
            fs::create_dir_all(destination)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn safe_relative(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn managed_keys() -> Vec<&'static str> {
    vec![
        "workbench.colorTheme",
        "editor.fontFamily",
        "terminal.integrated.fontFamily",
        "chat.editor.fontFamily",
        "debug.console.fontFamily",
        "markdown.preview.fontFamily",
        "terminal.integrated.enableImages",
        "workbench.startupEditor",
        "workbench.secondarySideBar.defaultVisibility",
        "chat.commandCenter.enabled",
        "workbench.tips.enabled",
        "workbench.welcomePage.walkthroughs.openOnInstall",
        "window.commandCenter",
        "window.title",
        "editor.smoothScrolling",
        "workbench.list.smoothScrolling",
        "terminal.integrated.smoothScrolling",
        "update.mode",
        "telemetry.telemetryLevel",
        "workbench.enableExperiments",
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    fn fixture() -> (TempDir, Editor, ProfilePaths) {
        let root = TempDir::new().unwrap();
        let user = root.path().join("source/User");
        let extensions = root.path().join("source/extensions");
        fs::create_dir_all(&user).unwrap();
        fs::create_dir_all(&extensions).unwrap();
        let editor = Editor {
            name: "Code".into(),
            user_dir: user,
            extensions_dir: Some(extensions),
        };
        let paths = ProfilePaths::from_environment(root.path(), &BTreeMap::new());
        (root, editor, paths)
    }

    #[test]
    fn imports_settings_keybindings_snippets_tasks_and_extensions() {
        let (_root, editor, paths) = fixture();
        fs::write(
            editor.user_dir.join("settings.json"),
            "{\n // mine\n \"editor.fontSize\": 19, \"workbench.colorTheme\": \"Old\",\n}\n",
        )
        .unwrap();
        fs::write(
            editor.user_dir.join("keybindings.json"),
            "[ {\"key\":\"cmd+k\",\"command\":\"mine\"}, ]",
        )
        .unwrap();
        fs::create_dir(editor.user_dir.join("snippets")).unwrap();
        fs::write(editor.user_dir.join("snippets/rust.json"), "{}").unwrap();
        fs::write(editor.user_dir.join("tasks.json"), "{}").unwrap();
        let extensions = editor.extensions_dir.as_ref().unwrap();
        fs::create_dir(extensions.join("acme-1.0.0")).unwrap();
        fs::write(extensions.join("acme-1.0.0/package.json"), "{}").unwrap();
        fs::write(
            extensions.join("extensions.json"),
            serde_json::to_vec(&json!([{
                "identifier": {"id": "acme.extension"},
                "version": "1.0.0",
                "relativeLocation": "acme-1.0.0"
            }]))
            .unwrap(),
        )
        .unwrap();

        let mut progress = Vec::new();
        let report = run_import_with_progress(&editor, &paths, &mut |done, total, id| {
            progress.push((done, total, id.to_owned()));
        });
        assert_eq!(progress, [(1, 1, "acme.extension".into())]);
        assert_eq!(report.settings.as_ref().unwrap().imported, 2);
        assert_eq!(
            report.settings.as_ref().unwrap().kept_by_tode,
            ["workbench.colorTheme"]
        );
        assert_eq!(report.keybindings, Some(1));
        assert_eq!(report.snippets, ["rust.json"]);
        assert!(report.tasks);
        assert_eq!(report.extensions.copied, ["acme.extension"]);
        let settings = fs::read_to_string(paths.user.join("settings.json")).unwrap();
        assert_eq!(
            tode_core::read_key(&settings, "editor.fontSize"),
            Some(json!(19))
        );
        assert_eq!(
            tode_core::read_key(&settings, "workbench.colorTheme"),
            Some(json!("Terminal Code"))
        );
        assert!(paths.extensions.join("acme-1.0.0/package.json").is_file());
    }

    #[test]
    fn deduplicates_keybindings_and_preserves_existing_extension_registry() {
        let (_root, editor, paths) = fixture();
        fs::create_dir_all(&paths.user).unwrap();
        let binding = r#"[{"key":"cmd+k","command":"mine"}]"#;
        fs::write(editor.user_dir.join("keybindings.json"), binding).unwrap();
        fs::write(paths.user.join("keybindings.json"), binding).unwrap();
        fs::write(
            editor
                .extensions_dir
                .as_ref()
                .unwrap()
                .join("extensions.json"),
            "[]",
        )
        .unwrap();
        let report = run_import(&editor, &paths);
        assert_eq!(report.keybindings, Some(0));
    }

    #[test]
    fn reports_missing_and_unsafe_extension_folders() {
        let (_root, editor, paths) = fixture();
        fs::write(
            editor
                .extensions_dir
                .as_ref()
                .unwrap()
                .join("extensions.json"),
            serde_json::to_vec(&json!([
                {"identifier":{"id":"missing"},"version":"1","relativeLocation":"missing-1"},
                {"identifier":{"id":"unsafe"},"version":"1","relativeLocation":"../escape"}
            ]))
            .unwrap(),
        )
        .unwrap();
        let report = run_import(&editor, &paths);
        assert_eq!(report.extensions.skipped.len(), 2);
        assert_eq!(report.extensions.skipped[0].why, "its folder is missing");
        assert_eq!(report.extensions.skipped[1].why, "its folder is unsafe");
    }

    #[test]
    fn rejects_symlinked_extension_tree() {
        use std::os::unix::fs::symlink;

        let (root, editor, paths) = fixture();
        let extensions = editor.extensions_dir.as_ref().unwrap();
        fs::create_dir(extensions.join("linked-1")).unwrap();
        fs::write(root.path().join("outside"), "secret").unwrap();
        symlink(
            root.path().join("outside"),
            extensions.join("linked-1/link"),
        )
        .unwrap();
        fs::write(
            extensions.join("extensions.json"),
            serde_json::to_vec(&json!([{
                "identifier":{"id":"linked"},"version":"1","relativeLocation":"linked-1"
            }]))
            .unwrap(),
        )
        .unwrap();
        let report = run_import(&editor, &paths);
        assert_eq!(report.extensions.skipped[0].why, "could not be copied");
        assert!(!paths.extensions.join("linked-1/link").exists());
    }

    #[test]
    fn discovers_editors_extensions_and_contents() {
        let root = TempDir::new().unwrap();
        let support = root.path().join(".config/Code/User/globalStorage");
        fs::create_dir_all(&support).unwrap();
        fs::write(support.join("state.vscdb"), "state").unwrap();
        let user = root.path().join(".config/Code/User");
        fs::write(user.join("settings.json"), "{}").unwrap();
        fs::create_dir(user.join("snippets")).unwrap();
        fs::write(user.join("snippets/rust.code-snippets"), "{}").unwrap();
        let extensions = root.path().join(".vscode/extensions");
        fs::create_dir_all(&extensions).unwrap();
        fs::write(extensions.join("extensions.json"), "[{},{}]").unwrap();

        let editors = find_editors(root.path(), None, false);
        assert_eq!(editors.len(), 1);
        assert_eq!(editors[0].name, "Code");
        assert_eq!(
            editors[0].extensions_dir.as_deref(),
            Some(extensions.as_path())
        );
        let contents = describe(&editors[0]);
        assert_eq!(
            contents,
            EditorContents {
                settings: true,
                keybindings: false,
                snippets: 1,
                tasks: false,
                extensions: 2,
            }
        );
        assert_eq!(
            summarise(&contents),
            "2 extensions, settings, 1 snippet files"
        );
    }

    #[test]
    fn discovery_uses_absolute_xdg_and_ignores_uninitialized_directories() {
        let root = TempDir::new().unwrap();
        let xdg = root.path().join("xdg");
        fs::create_dir_all(xdg.join("VSCodium/User/globalStorage")).unwrap();
        fs::write(xdg.join("VSCodium/User/globalStorage/state.vscdb"), "state").unwrap();
        fs::create_dir_all(xdg.join("Noise/User")).unwrap();
        assert_eq!(find_editors(root.path(), Some(&xdg), false).len(), 1);
        assert!(find_editors(root.path(), Some(Path::new("relative")), false).is_empty());
    }
}
