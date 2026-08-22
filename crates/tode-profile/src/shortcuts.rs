use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tode_core::{
    Binding, Decision, DecisionChoice, Decisions, FreedMove, canonical_chord, claim_bindings,
    fallback_bindings, ghostty_from_trigger, ghostty_keybinds_file, ghostty_to_trigger,
    ghostty_with_include, ghostty_without_include, hint_bindings, kitty_from_trigger,
    kitty_keybinds_file, kitty_to_trigger, kitty_with_include, kitty_with_shared_rebinds,
    kitty_without_include, override_bindings, parse_ghostty_trigger, parse_jsonc, quit_bindings,
};

use crate::{ProfilePaths, write_if_changed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TerminalKind {
    Ghostty,
    Kitty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalProvider {
    pub kind: TerminalKind,
    pub name: &'static str,
    pub binary: PathBuf,
    pub config_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorHold {
    pub command: String,
    pub guard: Option<String>,
    pub claimant: String,
    pub describes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedResolution {
    pub action: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutConflict {
    #[serde(rename = "editorId")]
    pub editor_id: String,
    pub trigger: String,
    pub current: Option<String>,
    pub editor: EditorHold,
    pub others: Vec<EditorHold>,
    #[serde(rename = "inTerminal")]
    pub in_terminal: String,
    pub short: String,
    pub freed: String,
    pub tradeoff: String,
    pub shared: Option<SharedResolution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedConflict {
    pub key: String,
    pub builtin: String,
    pub command: String,
    pub claimant: String,
    pub describes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutScan {
    pub terminal: Vec<ShortcutConflict>,
    pub imported: Vec<ImportedConflict>,
    pub occupied: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoOutcome {
    pub terminal_changed: bool,
    pub had_decisions: bool,
    pub keybindings_changed: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ShortcutError {
    #[error("{0}")]
    NotReady(String),
    #[error("run {program}: {message}")]
    Command { program: String, message: String },
    #[error("parse {0} keymap: {1}")]
    Parse(&'static str, String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

const GHOSTTY_HARMLESS: [&str; 14] = [
    "unbind",
    "ignore",
    "text:",
    "csi:",
    "esc:",
    "copy_to_clipboard",
    "paste_from_clipboard",
    "paste_from_selection",
    "adjust_selection",
    "scroll_to_selection",
    "search_selection",
    "end_search",
    "navigate_search",
    "jump_to_prompt",
];
const KITTY_HARMLESS: [&str; 11] = [
    "copy_or_noop",
    "scroll_line_up",
    "scroll_line_down",
    "scroll_page_up",
    "scroll_page_down",
    "scroll_home",
    "scroll_end",
    "scroll_to_prompt",
    "scroll_prompt_to_top",
    "scroll_prompt_to_bottom",
    "noop",
];
const KITTY_DUMP_SCRIPT: &str = r#"
def main():
    import json
    from kitty.actions import get_all_actions
    from kitty.config import load_config
    from kitty.constants import defconf
    from kitty.fast_data_types import SingleKey
    from kitty.types import human_repr_of_single_key
    def spell(k):
        name = human_repr_of_single_key(SingleKey(mods=k.mods, is_native=k.is_native, key=k.key), 0)
        return name[:-1] + 'plus' if name.endswith('++') or name == '+' else name
    opts = load_config(defconf)
    binds = []
    for key, defs in opts.keyboard_modes[""].keymap.items():
        matches = list(defs)
        if any(d.is_sequence for d in matches):
            last_terminal = -1
            for i, d in enumerate(matches):
                if not d.rest: last_terminal = i
            if last_terminal > -1:
                matches = matches[last_terminal:] if last_terminal == len(matches) - 1 else matches[last_terminal + 1:]
        else:
            matches = matches[-1:]
        entry = {"trigger": spell(key), "action": None, "sequences": []}
        for d in matches:
            if d.is_sequence and d.definition:
                entry["sequences"].append({"keys": " > ".join(spell(k) for k in (d.trigger,) + d.rest), "action": d.definition})
            elif not d.is_sequence and d.definition:
                entry["action"] = d.definition
        if entry["action"] or entry["sequences"]: binds.append(entry)
    docs = {}
    for group in get_all_actions().values():
        for action in group: docs[action.name] = action.short_help
    print(json.dumps({"binds": binds, "docs": docs}))
main()
"#;

#[derive(Debug, Clone, Deserialize)]
struct KittyBind {
    trigger: String,
    action: Option<String>,
    #[serde(default)]
    sequences: Vec<KittySequence>,
}

#[derive(Debug, Clone, Deserialize)]
struct KittySequence {
    keys: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KittyKeymap {
    binds: Vec<KittyBind>,
    #[serde(default)]
    docs: BTreeMap<String, String>,
}

pub fn detect_provider(
    home: &Path,
    environment: &BTreeMap<OsString, OsString>,
) -> Option<TerminalProvider> {
    if environment
        .get(OsStr::new("TERM_PROGRAM"))
        .is_some_and(|value| value == "ghostty")
        || environment.contains_key(OsStr::new("GHOSTTY_RESOURCES_DIR"))
    {
        return Some(provider(
            TerminalKind::Ghostty,
            "Ghostty",
            "ghostty",
            ghostty_config_dir(home, environment),
            environment,
        ));
    }
    if environment
        .get(OsStr::new("TERM"))
        .is_some_and(|value| value == "xterm-kitty")
        || environment.contains_key(OsStr::new("KITTY_WINDOW_ID"))
        || environment.contains_key(OsStr::new("KITTY_PID"))
    {
        return Some(provider(
            TerminalKind::Kitty,
            "kitty",
            "kitty",
            kitty_config_dir(home, environment),
            environment,
        ));
    }
    None
}

pub fn provider_readiness(provider: &TerminalProvider) -> Option<String> {
    if is_executable(&provider.binary) {
        return None;
    }
    Some(match provider.kind {
        TerminalKind::Ghostty => {
            "the ghostty cli is not on PATH, so its keybinds cannot be read".to_owned()
        }
        TerminalKind::Kitty => {
            "the kitty cli is not on PATH, so its keymap cannot be read".to_owned()
        }
    })
}

fn provider(
    kind: TerminalKind,
    name: &'static str,
    command: &'static str,
    config_dir: PathBuf,
    environment: &BTreeMap<OsString, OsString>,
) -> TerminalProvider {
    TerminalProvider {
        kind,
        name,
        binary: find_executable(command, environment).unwrap_or_else(|| command.into()),
        config_dir,
    }
}

pub fn scan_shortcuts(
    provider: &TerminalProvider,
    paths: &ProfilePaths,
) -> Result<ShortcutScan, ShortcutError> {
    if let Some(reason) = provider_readiness(provider) {
        return Err(ShortcutError::NotReady(reason));
    }
    let holders = editor_holders(paths);
    let decisions = load_decisions(paths);
    let (terminal, occupied) = match provider.kind {
        TerminalKind::Ghostty => scan_ghostty(provider, &holders, decisions.as_ref())?,
        TerminalKind::Kitty => scan_kitty(provider, &holders, decisions.as_ref())?,
    };
    Ok(ShortcutScan {
        terminal,
        imported: imported_conflicts(paths, &holders),
        occupied,
    })
}

pub fn auto_apply_shared(
    provider: &TerminalProvider,
    paths: &ProfilePaths,
    scan: &ShortcutScan,
) -> Result<bool, ShortcutError> {
    let mut decisions = load_decisions(paths).unwrap_or_else(|| Decisions {
        version: 1,
        terminal: terminal_id(provider.kind).to_owned(),
        choices: BTreeMap::new(),
    });
    let mut changed = false;
    for conflict in &scan.terminal {
        if conflict.shared.is_some()
            && conflict.current.is_some()
            && !decisions.choices.contains_key(&conflict.editor_id)
        {
            decisions.choices.insert(
                conflict.editor_id.clone(),
                Decision {
                    choice: DecisionChoice::Terminal,
                    key: None,
                    action: conflict.current.clone(),
                    guard: None,
                    owner_terminal: false,
                    command: None,
                },
            );
            changed = true;
        }
    }
    if changed {
        apply_decisions(provider, paths, &scan.terminal, &decisions)?;
    }
    Ok(changed)
}

pub fn apply_decisions(
    provider: &TerminalProvider,
    paths: &ProfilePaths,
    conflicts: &[ShortcutConflict],
    decisions: &Decisions,
) -> Result<bool, ShortcutError> {
    let mut moves = Vec::new();
    for conflict in conflicts {
        let Some(decision) = decisions.choices.get(&conflict.editor_id) else {
            continue;
        };
        if decision.choice == DecisionChoice::Terminal {
            moves.push(FreedMove {
                trigger: conflict.trigger.clone(),
                to: decision.key.clone(),
                action: decision.action.clone(),
                emit: None,
            });
        }
    }
    for (id, decision) in &decisions.choices {
        if !id.starts_with("claim:")
            || !decision.owner_terminal
            || decision.choice != DecisionChoice::Terminal
        {
            continue;
        }
        let Some(action) = decision.action.clone() else {
            continue;
        };
        let rest = &id["claim:".len()..];
        let chord = rest.split(':').next().unwrap_or(rest);
        moves.push(FreedMove {
            trigger: match provider.kind {
                TerminalKind::Ghostty => ghostty_to_trigger(chord),
                TerminalKind::Kitty => kitty_to_trigger(chord),
            },
            to: decision.key.clone(),
            action: Some(action),
            emit: None,
        });
    }
    let terminal_changed = write_provider_moves(provider, &moves)?;
    save_decisions(paths, decisions)?;
    let bindings_changed = install_shortcut_keybindings(paths, Some(decisions))?;
    Ok(terminal_changed || bindings_changed)
}

pub fn undo_shortcuts(
    provider: &TerminalProvider,
    paths: &ProfilePaths,
) -> Result<UndoOutcome, ShortcutError> {
    let had_decisions = decisions_file(paths).is_file();
    let terminal_changed = remove_provider_moves(provider)?;
    if had_decisions {
        fs::remove_file(decisions_file(paths))?;
    }
    let keybindings_changed = install_shortcut_keybindings(paths, None)?;
    Ok(UndoOutcome {
        terminal_changed,
        had_decisions,
        keybindings_changed,
    })
}

pub fn load_decisions(paths: &ProfilePaths) -> Option<Decisions> {
    fs::read(decisions_file(paths))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

pub fn save_decisions(paths: &ProfilePaths, decisions: &Decisions) -> Result<(), ShortcutError> {
    let mut contents = serde_json::to_vec_pretty(decisions)
        .map_err(|error| ShortcutError::Parse("decision", error.to_string()))?;
    contents.push(b'\n');
    write_if_changed(&decisions_file(paths), &contents)?;
    Ok(())
}

pub fn install_shortcut_keybindings(
    paths: &ProfilePaths,
    decisions: Option<&Decisions>,
) -> Result<bool, ShortcutError> {
    let file = paths.user.join("keybindings.json");
    let record = paths.data.join("keybindings.tode.json");
    let current = read_bindings(&file);
    let previous = read_bindings(&record);
    let mine = generated_bindings(decisions);
    let foreign: Vec<_> = current
        .into_iter()
        .filter(|entry| !previous.contains(entry) && !mine.contains(entry))
        .collect();
    let mut winners = decisions.map_or_else(Vec::new, |value| {
        let mut bindings = override_bindings(value, cfg!(target_os = "macos"));
        bindings.extend(claim_bindings(value));
        bindings
    });
    let quit = tode_core::quit_chord(cfg!(target_os = "macos"));
    let quit_claimed = decisions.is_some_and(|value| {
        value.choices.contains_key(&format!("import:{quit}"))
            || value.choices.contains_key(&format!("claim:{quit}"))
    });
    if !quit_claimed
        && foreign.iter().any(|entry| {
            canonical_chord(&entry.key) == quit
                && !entry.command.starts_with('-')
                && entry.command != "tode.confirmQuit"
        })
    {
        winners.push(Binding {
            key: quit.into(),
            command: "tode.confirmQuit".into(),
            when: Some(tode_core::quit_when(cfg!(target_os = "macos")).into()),
        });
    }
    let recorded: Vec<_> = mine.iter().chain(&winners).cloned().collect();
    let combined: Vec<_> = mine
        .iter()
        .chain(&foreign)
        .chain(&winners)
        .cloned()
        .collect();
    let mut record_bytes = serde_json::to_vec_pretty(&recorded)
        .map_err(|error| ShortcutError::Parse("keybinding record", error.to_string()))?;
    record_bytes.push(b'\n');
    write_if_changed(&record, &record_bytes)?;
    let mut output = b"// generated entries are reconciled by tode\n".to_vec();
    output.extend(
        serde_json::to_vec_pretty(&combined)
            .map_err(|error| ShortcutError::Parse("keybindings", error.to_string()))?,
    );
    output.push(b'\n');
    Ok(write_if_changed(&file, &output)?)
}

pub(crate) fn generated_bindings(decisions: Option<&Decisions>) -> Vec<Binding> {
    let mut mine = quit_bindings(decisions, cfg!(target_os = "macos"));
    mine.extend(hint_bindings(cfg!(target_os = "macos")));
    mine.extend(fallback_bindings(decisions));
    mine
}

fn read_bindings(path: &Path) -> Vec<Binding> {
    fs::read_to_string(path)
        .ok()
        .and_then(|source| parse_jsonc(&source))
        .unwrap_or_default()
}

fn scan_ghostty(
    provider: &TerminalProvider,
    holders: &BTreeMap<String, Vec<EditorHold>>,
    decisions: Option<&Decisions>,
) -> Result<(Vec<ShortcutConflict>, BTreeMap<String, String>), ShortcutError> {
    let output = run(&provider.binary, &["+list-keybinds"])?;
    let docs = run(&provider.binary, &["+list-actions", "--docs"])
        .map(|source| parse_action_docs(&source))
        .unwrap_or_default();
    let freed = ghostty_freed(&provider.config_dir);
    let mut seen = BTreeSet::new();
    let mut conflicts = Vec::new();
    let mut occupied = BTreeMap::new();
    for line in output.lines() {
        let Some(rest) = line.trim().strip_prefix("keybind") else {
            continue;
        };
        let Some((raw, action)) = rest.trim_start().strip_prefix('=').and_then(|rest| {
            let (raw, action) = rest.split_once('=')?;
            Some((raw.trim(), action.trim()))
        }) else {
            continue;
        };
        let parsed = parse_ghostty_trigger(raw);
        if parsed.passes_through || ghostty_harmless(action) {
            continue;
        }
        let effective = (!freed.contains(&parsed.trigger)).then_some(action.to_owned());
        if let (Some(chord), Some(action)) =
            (ghostty_from_trigger(&parsed.trigger), effective.as_ref())
        {
            occupied.insert(chord, action.clone());
        }
        consider_ghostty(
            &parsed.trigger,
            effective,
            holders,
            decisions,
            &docs,
            &mut seen,
            &mut conflicts,
        );
    }
    for trigger in freed {
        consider_ghostty(
            &trigger,
            None,
            holders,
            decisions,
            &docs,
            &mut seen,
            &mut conflicts,
        );
    }
    Ok((conflicts, occupied))
}

fn consider_ghostty(
    trigger: &str,
    current: Option<String>,
    holders: &BTreeMap<String, Vec<EditorHold>>,
    decisions: Option<&Decisions>,
    docs: &BTreeMap<String, String>,
    seen: &mut BTreeSet<String>,
    conflicts: &mut Vec<ShortcutConflict>,
) {
    let Some(chord) = ghostty_from_trigger(trigger) else {
        return;
    };
    if !seen.insert(chord.clone()) {
        return;
    }
    let Some(found) = holders.get(&chord).filter(|holds| !holds.is_empty()) else {
        return;
    };
    let current = current.or_else(|| decided_action(decisions, &chord));
    let doing = current
        .as_deref()
        .map(words)
        .unwrap_or_else(|| "what it ran before".into());
    let described = current
        .as_deref()
        .and_then(|action| docs.get(action.split(':').next().unwrap_or(action)))
        .cloned()
        .unwrap_or_else(|| doing.clone());
    let primary = found[0].clone();
    let means = primary
        .describes
        .clone()
        .unwrap_or_else(|| words(&primary.command));
    conflicts.push(ShortcutConflict {
        editor_id: chord,
        trigger: trigger.into(),
        current,
        editor: primary,
        others: found[1..].to_vec(),
        in_terminal: format!("runs {doing} in Ghostty, so {means} never reaches the editor"),
        short: described,
        freed: format!("{doing} goes"),
        tradeoff: format!("Ghostty's {doing} stops working"),
        shared: None,
    });
}
fn parse_action_docs(source: &str) -> BTreeMap<String, String> {
    let mut docs = BTreeMap::new();
    let mut action = None;
    let mut lines = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let heading = (!line.starts_with(char::is_whitespace))
            .then(|| trimmed.strip_suffix(':'))
            .flatten()
            .filter(|name| {
                !name.is_empty()
                    && name.chars().all(|character| {
                        character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '_'
                    })
            });
        if let Some(heading) = heading {
            insert_action_doc(&mut docs, action.take(), &mut lines);
            action = Some(heading.to_owned());
        } else if action.is_some() && line.starts_with(char::is_whitespace) && !trimmed.is_empty() {
            lines.push(trimmed.to_owned());
        }
    }
    insert_action_doc(&mut docs, action, &mut lines);
    docs
}

fn insert_action_doc(
    docs: &mut BTreeMap<String, String>,
    action: Option<String>,
    lines: &mut Vec<String>,
) {
    let Some(action) = action else {
        lines.clear();
        return;
    };
    if lines.is_empty() {
        return;
    }
    let text = lines.join(" ");
    let sentence = text
        .split_once(". ")
        .map_or(text.as_str(), |(sentence, _)| sentence)
        .trim_end_matches('.');
    docs.insert(action, sentence.to_owned());
    lines.clear();
}

fn scan_kitty(
    provider: &TerminalProvider,
    holders: &BTreeMap<String, Vec<EditorHold>>,
    decisions: Option<&Decisions>,
) -> Result<(Vec<ShortcutConflict>, BTreeMap<String, String>), ShortcutError> {
    let output = run(&provider.binary, &["+runpy", KITTY_DUMP_SCRIPT])?;
    let keymap: KittyKeymap = serde_json::from_str(&output)
        .map_err(|error| ShortcutError::Parse("kitty", error.to_string()))?;
    let freed = kitty_freed(&provider.config_dir);
    let mut seen = BTreeSet::new();
    let mut conflicts = Vec::new();
    let mut occupied = BTreeMap::new();
    for binding in &keymap.binds {
        if binding.action.as_deref().is_some_and(kitty_harmless) {
            continue;
        }
        let effective = (!freed.contains(&binding.trigger)).then_some(binding);
        if let (Some(chord), Some(binding)) = (kitty_from_trigger(&binding.trigger), effective) {
            occupied.insert(
                chord,
                binding
                    .action
                    .clone()
                    .unwrap_or_else(|| "key sequence".into()),
            );
        }
        consider_kitty(
            &binding.trigger,
            effective,
            &keymap.docs,
            holders,
            decisions,
            &mut seen,
            &mut conflicts,
        );
    }
    for trigger in freed {
        consider_kitty(
            &trigger,
            None,
            &keymap.docs,
            holders,
            decisions,
            &mut seen,
            &mut conflicts,
        );
    }
    Ok((conflicts, occupied))
}

#[allow(clippy::too_many_arguments)]
fn consider_kitty(
    trigger: &str,
    binding: Option<&KittyBind>,
    docs: &BTreeMap<String, String>,
    holders: &BTreeMap<String, Vec<EditorHold>>,
    decisions: Option<&Decisions>,
    seen: &mut BTreeSet<String>,
    conflicts: &mut Vec<ShortcutConflict>,
) {
    let Some(chord) = kitty_from_trigger(trigger) else {
        return;
    };
    if !seen.insert(chord.clone()) {
        return;
    }
    let Some(found) = holders.get(&chord).filter(|holds| !holds.is_empty()) else {
        return;
    };
    let current = binding
        .and_then(|value| value.action.clone())
        .or_else(|| decided_action(decisions, &chord));
    let doing = current.as_deref().map(words).unwrap_or_else(|| {
        binding.map_or_else(
            || "what it ran before".into(),
            |value| format!("{} key sequences", value.sequences.len()),
        )
    });
    let stored_current = binding.map_or_else(
        || current.clone(),
        |value| {
            value.action.clone().or_else(|| {
                Some(format!(
                    "key sequence ({})",
                    value
                        .sequences
                        .iter()
                        .map(|sequence| sequence.keys.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
        },
    );
    let primary = found[0].clone();
    let means = primary
        .describes
        .clone()
        .unwrap_or_else(|| words(&primary.command));
    let shared_action = current
        .as_deref()
        .and_then(|action| action.split_whitespace().next())
        .filter(|action| *action == "copy_to_clipboard")
        .map(|_| "copy_or_noop".to_owned());
    let doc = current
        .as_deref()
        .and_then(|action| action.split_whitespace().next())
        .and_then(|action| docs.get(action));
    conflicts.push(ShortcutConflict {
        editor_id: chord,
        trigger: trigger.into(),
        current: stored_current,
        editor: primary,
        others: found[1..].to_vec(),
        in_terminal: format!(
            "runs {doing} in kitty{}, so {means} never reaches the editor",
            doc.map_or_else(String::new, |text| format!(" ({})", lower_first(text)))
        ),
        short: doing.clone(),
        freed: shared_action
            .as_ref()
            .map_or_else(|| format!("{doing} goes"), |_| format!("{doing} stays whenever kitty can act")),
        tradeoff: shared_action
            .as_ref()
            .map_or_else(|| format!("kitty's {doing} stops working"), |_| "none — the chord is shared".into()),
        shared: shared_action.map(|action| SharedResolution {
            note: format!(
                "kitty currently swallows {trigger} even when it has nothing to copy. Rebinding it to {action} keeps kitty's {doing} whenever kitty has its own selection and passes the chord through to {means} otherwise."
            ),
            action,
        }),
    });
}

pub(crate) fn editor_holders(paths: &ProfilePaths) -> BTreeMap<String, Vec<EditorHold>> {
    let decisions = load_decisions(paths);
    let mine = generated_bindings(decisions.as_ref());
    let previous = read_bindings(&paths.data.join("keybindings.tode.json"));
    let current = read_bindings(&paths.user.join("keybindings.json"));
    let foreign: Vec<_> = current
        .into_iter()
        .filter(|binding| !previous.contains(binding) && !mine.contains(binding))
        .collect();
    let mut holders: BTreeMap<String, Vec<EditorHold>> = BTreeMap::new();
    for (binding, claimant) in foreign
        .iter()
        .map(|binding| (binding, "imported"))
        .chain(mine.iter().map(|binding| (binding, "terminal-code")))
    {
        record_holder(&mut holders, binding, claimant, None);
    }
    for (chord, binding) in default_bindings() {
        if !removal_masked(&foreign, &chord, &binding.command) {
            record_holder(&mut holders, &binding, "terminal-code", None);
        }
    }
    for (chord, claim) in extension_bindings(paths) {
        holders.entry(chord).or_default().push(claim);
    }
    for values in holders.values_mut() {
        let mut commands = BTreeSet::new();
        values.retain(|hold| commands.insert(hold.command.clone()));
        values.sort_by_key(|hold| hold.guard.is_some());
    }
    holders
}

fn record_holder(
    holders: &mut BTreeMap<String, Vec<EditorHold>>,
    binding: &Binding,
    claimant: &str,
    describes: Option<String>,
) {
    if binding.key.is_empty() || binding.command.is_empty() || binding.command.starts_with('-') {
        return;
    }
    holders
        .entry(canonical_chord(&binding.key))
        .or_default()
        .push(EditorHold {
            command: binding.command.clone(),
            guard: binding.when.clone(),
            claimant: claimant.into(),
            describes,
        });
}

fn default_bindings() -> BTreeMap<String, Binding> {
    let source = if cfg!(target_os = "macos") {
        include_str!("../../../assets/keymaps/vscode-mac.json")
    } else {
        include_str!("../../../assets/keymaps/vscode-linux.json")
    };
    let parsed: Value =
        serde_json::from_str(source).expect("committed VS Code keymap is valid JSON");
    let mut grouped: BTreeMap<String, Vec<Binding>> = BTreeMap::new();
    for value in parsed["bindings"].as_array().into_iter().flatten() {
        let Some(key) = value["key"].as_str() else {
            continue;
        };
        let Some(command) = value["command"]
            .as_str()
            .filter(|value| !value.starts_with('-'))
        else {
            continue;
        };
        grouped
            .entry(canonical_chord(key))
            .or_default()
            .push(Binding {
                key: key.into(),
                command: command.into(),
                when: value["when"].as_str().map(str::to_owned),
            });
    }
    grouped
        .into_iter()
        .filter_map(|(chord, entries)| choose_default(entries).map(|entry| (chord, entry)))
        .collect()
}

fn choose_default(entries: Vec<Binding>) -> Option<Binding> {
    let core = |entry: &Binding| {
        [
            "editor.",
            "workbench.",
            "actions.",
            "cursor",
            "list.",
            "search.",
        ]
        .iter()
        .any(|prefix| entry.command.starts_with(prefix))
    };
    entries
        .iter()
        .find(|entry| entry.when.is_none() && core(entry))
        .or_else(|| entries.iter().find(|entry| entry.when.is_none()))
        .or_else(|| entries.iter().find(|entry| core(entry)))
        .or_else(|| entries.first())
        .cloned()
}

fn extension_bindings(paths: &ProfilePaths) -> Vec<(String, EditorHold)> {
    let mut claims = Vec::new();
    for folder in list_dir_paths(&paths.extensions) {
        let Ok(source) = fs::read_to_string(folder.join("package.json")) else {
            continue;
        };
        let Ok(package) = serde_json::from_str::<Value>(&source) else {
            continue;
        };
        let claimant = package["displayName"]
            .as_str()
            .or_else(|| package["name"].as_str())
            .unwrap_or("extension")
            .to_owned();
        let titles: BTreeMap<_, _> = package
            .pointer("/contributes/commands")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|command| Some((command["command"].as_str()?, command["title"].as_str()?)))
            .map(|(command, title)| (command.to_owned(), title.to_owned()))
            .collect();
        for binding in package
            .pointer("/contributes/keybindings")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let platform_key = if cfg!(target_os = "macos") {
                "mac"
            } else {
                "linux"
            };
            let Some(key) = binding[platform_key]
                .as_str()
                .or_else(|| binding["key"].as_str())
            else {
                continue;
            };
            let Some(command) = binding["command"].as_str() else {
                continue;
            };
            claims.push((
                canonical_chord(key),
                EditorHold {
                    command: command.into(),
                    guard: binding["when"].as_str().map(str::to_owned),
                    claimant: claimant.clone(),
                    describes: titles.get(command).cloned(),
                },
            ));
        }
    }
    claims
}

fn imported_conflicts(
    paths: &ProfilePaths,
    holders: &BTreeMap<String, Vec<EditorHold>>,
) -> Vec<ImportedConflict> {
    let foreign = read_bindings(&paths.user.join("keybindings.json"));
    let builtins = generated_bindings(None);
    let mut output = Vec::new();
    let mut seen = BTreeSet::new();
    for builtin in builtins {
        let chord = canonical_chord(&builtin.key);
        if !seen.insert(chord.clone()) {
            continue;
        }
        let found = foreign.iter().find(|binding| {
            !binding.command.starts_with('-')
                && canonical_chord(&binding.key) == chord
                && binding.command != builtin.command
        });
        if let Some(binding) = found {
            output.push(ImportedConflict {
                key: builtin.key,
                builtin: builtin.command,
                command: binding.command.clone(),
                claimant: "imported".into(),
                describes: None,
            });
            continue;
        }
        if let Some(claim) = holders.get(&chord).and_then(|values| {
            values
                .iter()
                .find(|claim| claim.claimant != "terminal-code" && claim.command != builtin.command)
        }) {
            output.push(ImportedConflict {
                key: builtin.key,
                builtin: builtin.command,
                command: claim.command.clone(),
                claimant: claim.claimant.clone(),
                describes: claim.describes.clone(),
            });
        }
    }
    output
}

fn write_provider_moves(
    provider: &TerminalProvider,
    moves: &[FreedMove],
) -> Result<bool, ShortcutError> {
    if moves.is_empty() {
        return remove_provider_moves(provider);
    }
    let (config_name, owned_name, config, owned) = match provider.kind {
        TerminalKind::Ghostty => (
            "config",
            "keybinds.ghostty",
            ghostty_with_include(
                &fs::read_to_string(provider.config_dir.join("config")).unwrap_or_default(),
            ),
            ghostty_keybinds_file(moves),
        ),
        TerminalKind::Kitty => {
            let moves = kitty_with_shared_rebinds(moves);
            (
                "kitty.conf",
                "keybinds.kitty.conf",
                kitty_with_include(
                    &fs::read_to_string(provider.config_dir.join("kitty.conf")).unwrap_or_default(),
                ),
                kitty_keybinds_file(&moves),
            )
        }
    };
    let changed_owned = write_if_changed(
        &provider.config_dir.join("tode").join(owned_name),
        owned.as_bytes(),
    )?;
    let changed_config =
        write_if_changed(&provider.config_dir.join(config_name), config.as_bytes())?;
    Ok(changed_owned || changed_config)
}

fn remove_provider_moves(provider: &TerminalProvider) -> Result<bool, ShortcutError> {
    let (config_name, owned_name) = match provider.kind {
        TerminalKind::Ghostty => ("config", "keybinds.ghostty"),
        TerminalKind::Kitty => ("kitty.conf", "keybinds.kitty.conf"),
    };
    let owned = provider.config_dir.join("tode").join(owned_name);
    let mut changed = false;
    if owned.is_file() {
        fs::remove_file(owned)?;
        changed = true;
    }
    let config_file = provider.config_dir.join(config_name);
    if let Ok(source) = fs::read_to_string(&config_file) {
        let stripped = match provider.kind {
            TerminalKind::Ghostty => ghostty_without_include(&source),
            TerminalKind::Kitty => kitty_without_include(&source),
        };
        if stripped != source {
            write_if_changed(&config_file, stripped.as_bytes())?;
            changed = true;
        }
    }
    Ok(changed)
}

fn ghostty_freed(config_dir: &Path) -> BTreeSet<String> {
    let source = fs::read_to_string(config_dir.join("tode/keybinds.ghostty")).unwrap_or_default();
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("keybind")?.trim_start();
            let (trigger, action) = rest.strip_prefix('=')?.split_once('=')?;
            let action = action.trim();
            (action == "unbind"
                || action.starts_with("esc:")
                || action.starts_with("csi:")
                || action.starts_with("text:"))
            .then(|| trigger.trim().to_owned())
        })
        .collect()
}

fn kitty_freed(config_dir: &Path) -> BTreeSet<String> {
    let source =
        fs::read_to_string(config_dir.join("tode/keybinds.kitty.conf")).unwrap_or_default();
    source
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            (parts.next()? == "map").then_some(())?;
            let trigger = parts.next()?;
            let action = parts.next();
            (action.is_none() || action == Some("copy_or_noop")).then(|| trigger.to_owned())
        })
        .collect()
}

fn decided_action(decisions: Option<&Decisions>, chord: &str) -> Option<String> {
    decisions
        .and_then(|value| value.choices.get(chord))
        .and_then(|decision| decision.action.clone())
        .or_else(|| {
            decisions.and_then(|value| {
                value.choices.iter().find_map(|(id, decision)| {
                    let owns = id == &format!("claim:{chord}")
                        || id.starts_with(&format!("claim:{chord}:"));
                    (owns && decision.owner_terminal)
                        .then(|| decision.action.clone())
                        .flatten()
                })
            })
        })
}

fn removal_masked(bindings: &[Binding], chord: &str, command: &str) -> bool {
    bindings.iter().any(|binding| {
        binding.command == format!("-{command}") && canonical_chord(&binding.key) == chord
    })
}

fn ghostty_harmless(action: &str) -> bool {
    GHOSTTY_HARMLESS.iter().any(|prefix| {
        if prefix.ends_with(':') {
            action.starts_with(prefix)
        } else {
            action == *prefix || action.starts_with(&format!("{prefix}:"))
        }
    })
}

fn kitty_harmless(action: &str) -> bool {
    KITTY_HARMLESS
        .iter()
        .any(|prefix| action == *prefix || action.starts_with(&format!("{prefix} ")))
}

fn run(program: &Path, arguments: &[&str]) -> Result<String, ShortcutError> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| ShortcutError::Command {
            program: program.display().to_string(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(ShortcutError::Command {
            program: program.display().to_string(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    String::from_utf8(output.stdout).map_err(|error| ShortcutError::Command {
        program: program.display().to_string(),
        message: error.to_string(),
    })
}

fn find_executable(name: &str, environment: &BTreeMap<OsString, OsString>) -> Option<PathBuf> {
    let path = environment.get(OsStr::new("PATH"))?;
    std::env::split_paths(path)
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
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
    if let Some(path) = absolute_environment_path(environment, "KITTY_CONFIG_DIRECTORY") {
        candidates.push(path);
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
    environment
        .get(OsStr::new(name))
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn decisions_file(paths: &ProfilePaths) -> PathBuf {
    paths.data.join("shortcuts.json")
}

fn terminal_id(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::Ghostty => "ghostty",
        TerminalKind::Kitty => "kitty",
    }
}

fn list_dir_paths(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

fn lower_first(value: &str) -> String {
    let mut characters = value.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_lowercase().collect::<String>() + characters.as_str()
    })
}

pub(crate) fn words(value: &str) -> String {
    let mut output = String::new();
    let mut previous_lower = false;
    for character in value.replace(['_', '.', ':'], " ").chars() {
        if character.is_uppercase() && previous_lower {
            output.push(' ');
        }
        output.extend(character.to_lowercase());
        previous_lower = character.is_lowercase();
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessInfo {
    ppid: i32,
    command: String,
}

pub fn reload_provider(provider: &TerminalProvider) -> bool {
    let signal = match provider.kind {
        TerminalKind::Ghostty => Signal::SIGUSR2,
        TerminalKind::Kitty => Signal::SIGUSR1,
    };
    reload_with(
        provider.kind,
        i32::try_from(std::process::id()).unwrap_or(i32::MAX),
        process_info,
        |pid| kill(Pid::from_raw(pid), signal).map_err(|_| ()),
    )
}

fn reload_with(
    kind: TerminalKind,
    start_pid: i32,
    mut info: impl FnMut(i32) -> Option<ProcessInfo>,
    mut signal: impl FnMut(i32) -> Result<(), ()>,
) -> bool {
    let wanted = match kind {
        TerminalKind::Ghostty => "ghostty",
        TerminalKind::Kitty => "kitty",
    };
    let mut pid = start_pid;
    for _ in 0..16 {
        let Some(current) = info(pid) else {
            return false;
        };
        if current.ppid <= 1 {
            return false;
        }
        let Some(parent) = info(current.ppid) else {
            return false;
        };
        let basename = Path::new(&parent.command)
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or(&parent.command);
        if basename.eq_ignore_ascii_case(wanted) {
            return signal(current.ppid).is_ok();
        }
        pid = current.ppid;
    }
    false
}

fn process_info(pid: i32) -> Option<ProcessInfo> {
    let output = Command::new("ps")
        .args(["-o", "ppid=,comm=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let source = String::from_utf8(output.stdout).ok()?;
    let source = source.trim();
    let split = source.find(char::is_whitespace)?;
    let ppid = source[..split].parse().ok()?;
    let command = source[split..].trim().to_owned();
    (!command.is_empty()).then_some(ProcessInfo { ppid, command })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn environment(root: &TempDir, terminal: &str) -> BTreeMap<OsString, OsString> {
        let bin = root.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let mut environment = BTreeMap::from([
            (OsString::from("PATH"), bin.into_os_string()),
            (
                OsString::from("XDG_CONFIG_HOME"),
                root.path().join("config").into_os_string(),
            ),
        ]);
        match terminal {
            "ghostty" => {
                environment.insert(OsString::from("TERM_PROGRAM"), OsString::from("ghostty"));
            }
            "kitty" => {
                environment.insert(OsString::from("TERM"), OsString::from("xterm-kitty"));
            }
            _ => {}
        }
        environment
    }

    fn executable(path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn detects_provider_and_reports_missing_cli() {
        let root = TempDir::new().unwrap();
        let environment = environment(&root, "ghostty");
        let provider = detect_provider(root.path(), &environment).unwrap();
        assert_eq!(
            provider_readiness(&provider).as_deref(),
            Some("the ghostty cli is not on PATH, so its keybinds cannot be read")
        );
        executable(&root.path().join("bin/ghostty"), "exit 0");
        let provider = detect_provider(root.path(), &environment).unwrap();
        assert_eq!(provider.kind, TerminalKind::Ghostty);
        assert_eq!(provider_readiness(&provider), None);
    }

    #[test]
    fn scans_ghostty_effective_keys_against_profile_holders() {
        let root = TempDir::new().unwrap();
        let environment = environment(&root, "ghostty");
        executable(
            &root.path().join("bin/ghostty"),
            "printf 'keybind = ctrl+c=copy_to_clipboard\\nkeybind = ctrl+p=new_tab\\n'",
        );
        let paths = ProfilePaths::from_environment(root.path(), &environment);
        fs::create_dir_all(&paths.user).unwrap();
        fs::write(
            paths.user.join("keybindings.json"),
            r#"[{"key":"ctrl+p","command":"workbench.action.quickOpen"}]"#,
        )
        .unwrap();
        let provider = detect_provider(root.path(), &environment).unwrap();
        let scan = scan_shortcuts(&provider, &paths).unwrap();
        assert_eq!(scan.terminal.len(), 1);
        assert_eq!(scan.terminal[0].editor_id, "ctrl+p");
        assert_eq!(scan.terminal[0].current.as_deref(), Some("new_tab"));
    }

    #[test]
    fn kitty_shared_copy_is_applied_once_and_undo_is_clean() {
        let root = TempDir::new().unwrap();
        let environment = environment(&root, "kitty");
        executable(
            &root.path().join("bin/kitty"),
            r#"printf '%s\n' '{"binds":[{"trigger":"ctrl+c","action":"copy_to_clipboard","sequences":[]}],"docs":{}}'"#,
        );
        let paths = ProfilePaths::from_environment(root.path(), &environment);
        let provider = detect_provider(root.path(), &environment).unwrap();
        let scan = scan_shortcuts(&provider, &paths).unwrap();
        assert_eq!(scan.terminal.len(), 1);
        assert!(scan.terminal[0].shared.is_some());
        assert!(auto_apply_shared(&provider, &paths, &scan).unwrap());
        let owned = provider.config_dir.join("tode/keybinds.kitty.conf");
        assert!(fs::read_to_string(&owned).unwrap().contains("copy_or_noop"));
        let first = fs::read(&owned).unwrap();
        assert!(!auto_apply_shared(&provider, &paths, &scan).unwrap());
        assert_eq!(fs::read(&owned).unwrap(), first);
        let undone = undo_shortcuts(&provider, &paths).unwrap();
        assert!(undone.terminal_changed);
        assert!(undone.had_decisions);
        assert!(!owned.exists());
    }

    #[test]
    fn keybinding_reconciliation_preserves_foreign_entries_byte_stably() {
        let root = TempDir::new().unwrap();
        let paths = ProfilePaths::from_environment(root.path(), &BTreeMap::new());
        fs::create_dir_all(&paths.user).unwrap();
        fs::write(
            paths.user.join("keybindings.json"),
            r#"[{"key":"cmd+k","command":"mine"}]"#,
        )
        .unwrap();
        assert!(install_shortcut_keybindings(&paths, None).unwrap());
        let first = fs::read(paths.user.join("keybindings.json")).unwrap();
        assert!(!install_shortcut_keybindings(&paths, None).unwrap());
        assert_eq!(
            fs::read(paths.user.join("keybindings.json")).unwrap(),
            first
        );
        let parsed: Vec<Binding> = parse_jsonc(&String::from_utf8(first).unwrap()).unwrap();
        assert!(parsed.iter().any(|binding| binding.command == "mine"));
    }
    #[test]
    fn reload_walks_ancestry_and_signals_terminal_parent() {
        let processes = BTreeMap::from([
            (
                100,
                ProcessInfo {
                    ppid: 90,
                    command: "tode".into(),
                },
            ),
            (
                90,
                ProcessInfo {
                    ppid: 80,
                    command: "/bin/zsh".into(),
                },
            ),
            (
                80,
                ProcessInfo {
                    ppid: 70,
                    command: "/Applications/Ghostty.app/Contents/MacOS/ghostty".into(),
                },
            ),
        ]);
        let mut signaled = None;
        assert!(reload_with(
            TerminalKind::Ghostty,
            100,
            |pid| processes.get(&pid).cloned(),
            |pid| {
                signaled = Some(pid);
                Ok(())
            },
        ));
        assert_eq!(signaled, Some(80));
    }

    #[test]
    fn reload_stops_after_sixteen_hops() {
        let mut signaled = false;
        assert!(!reload_with(
            TerminalKind::Kitty,
            100,
            |pid| Some(ProcessInfo {
                ppid: pid - 1,
                command: "/bin/shell".into(),
            }),
            |_| {
                signaled = true;
                Ok(())
            },
        ));
        assert!(!signaled);
    }
    #[test]
    fn parses_ghostty_action_docs_first_sentence() {
        let docs = parse_action_docs(
            "new_tab:\n  Opens a new tab. More detail follows.\n\nclose_tab:\n  Closes the current tab.\n",
        );
        assert_eq!(docs["new_tab"], "Opens a new tab");
        assert_eq!(docs["close_tab"], "Closes the current tab");
    }
}
