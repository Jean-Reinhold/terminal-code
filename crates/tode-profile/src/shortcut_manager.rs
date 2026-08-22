use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use tode_core::{Decision, DecisionChoice, Decisions, canonical_chord, normalize_chord};

use crate::ProfilePaths;
use crate::shortcuts::{
    EditorHold, ShortcutConflict, ShortcutError, ShortcutScan, TerminalProvider, apply_decisions,
    editor_holders, generated_bindings, load_decisions, scan_shortcuts, words,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManagerRowKind {
    Terminal,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerDetail {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerTerminal {
    pub name: String,
    pub short: String,
    pub does: String,
    pub freed: String,
    pub tradeoff: String,
    pub bound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerClaim {
    pub chord: String,
    pub command: String,
    pub claimant: String,
    pub describes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resting: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided: Option<Decision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerRow {
    pub id: String,
    pub kind: ManagerRowKind,
    pub means: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ManagerDetail>,
    pub terminal: ManagerTerminal,
    #[serde(rename = "importedCommand", skip_serializing_if = "Option::is_none")]
    pub imported_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimant: Option<String>,
    #[serde(rename = "claimDescribes", skip_serializing_if = "Option::is_none")]
    pub claim_describes: Option<String>,
    #[serde(rename = "claimDecision", skip_serializing_if = "Option::is_none")]
    pub claim_decision: Option<Decision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub claims: Vec<ManagerClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakenResult {
    pub holder: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim: Option<ManagerClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimInfo {
    pub command: String,
    pub when: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionKind {
    Terminal,
    Import,
    Claim,
}

#[derive(Debug)]
pub struct ShortcutSession {
    provider: TerminalProvider,
    paths: ProfilePaths,
    scan: ShortcutScan,
    holders: BTreeMap<String, Vec<EditorHold>>,
    pub staged: BTreeMap<String, Decision>,
}

#[derive(Debug, Clone)]
struct PendingClaim {
    claim: ManagerClaim,
    decision: Option<Decision>,
    exempt_command: Option<String>,
    exempt_action: Option<String>,
}

impl ShortcutSession {
    pub fn new(provider: TerminalProvider, paths: ProfilePaths) -> Result<Self, ShortcutError> {
        let scan = scan_shortcuts(&provider, &paths)?;
        let holders = editor_holders(&paths);
        let staged = load_decisions(&paths)
            .filter(|decisions| decisions.terminal == terminal_id(&provider))
            .map(|decisions| decisions.choices)
            .unwrap_or_default();
        Ok(Self {
            provider,
            paths,
            scan,
            holders,
            staged,
        })
    }

    pub fn provider(&self) -> &TerminalProvider {
        &self.provider
    }

    pub fn scan(&self) -> &ShortcutScan {
        &self.scan
    }

    pub fn normalize(&self, chord: &str) -> Option<String> {
        normalize_chord(chord)
    }

    pub fn rows(&mut self) -> Vec<ManagerRow> {
        self.refresh();
        let mut rows = Vec::new();
        let mut currents = BTreeMap::new();
        for conflict in self
            .scan
            .terminal
            .iter()
            .filter(|conflict| conflict.shared.is_none())
        {
            if let Some(current) = &conflict.current {
                currents.insert(conflict.editor_id.clone(), current.clone());
            }
            let legacy = self.staged.get(&format!("claim:{}", conflict.editor_id));
            let mirrored = conflict.current.as_ref().and_then(|current| {
                self.staged
                    .get(&format!("claim:{}:{current}", conflict.editor_id))
                    .or_else(|| {
                        legacy.filter(|decision| {
                            decision.owner_terminal && decision.action.as_ref() == Some(current)
                        })
                    })
            });
            let decision = self.staged.get(&conflict.editor_id).or_else(|| {
                mirrored.filter(|decision| decision.choice == DecisionChoice::Terminal)
            });
            let mut claims: Vec<_> = conflict
                .others
                .iter()
                .map(|other| ManagerClaim {
                    chord: conflict.editor_id.clone(),
                    command: other.command.clone(),
                    claimant: other.claimant.clone(),
                    describes: other
                        .describes
                        .clone()
                        .unwrap_or_else(|| words(&other.command)),
                    when: other.guard.clone(),
                    resting: Some(true),
                    decided: self.staged_about(&conflict.editor_id, &other.command, false),
                })
                .collect();
            if let Some(decision) = decision {
                let (exempt_command, exempt_action) = match decision.choice {
                    DecisionChoice::Editor => (
                        decision
                            .command
                            .clone()
                            .or_else(|| Some(conflict.editor.command.clone())),
                        None,
                    ),
                    _ => (None, conflict.current.clone()),
                };
                self.follow_decision(
                    &conflict.editor_id,
                    decision,
                    exempt_command,
                    exempt_action,
                    &mut claims,
                );
            }
            rows.push(ManagerRow {
                id: conflict.editor_id.clone(),
                kind: ManagerRowKind::Terminal,
                means: conflict
                    .editor
                    .describes
                    .clone()
                    .unwrap_or_else(|| words(&conflict.editor.command)),
                detail: Some(ManagerDetail {
                    command: conflict.editor.command.clone(),
                    when: conflict.editor.guard.clone(),
                }),
                terminal: terminal_row(&self.provider, conflict),
                imported_command: None,
                claimant: None,
                claim_describes: None,
                claim_decision: None,
                decision: decision.cloned(),
                claims: deduplicate_claims(claims),
            });
        }
        for imported in &self.scan.imported {
            let claim_decision = self
                .staged
                .get(&format!("claim:{}:{}", imported.key, imported.command))
                .or_else(|| self.staged.get(&format!("claim:{}", imported.key)));
            let decision = self.staged.get(&format!("import:{}", imported.key));
            let mut claims = Vec::new();
            if let Some(decision) = decision {
                self.follow_decision(
                    &imported.key,
                    decision,
                    decision
                        .command
                        .clone()
                        .or_else(|| Some(imported.command.clone())),
                    None,
                    &mut claims,
                );
            }
            if let Some(decision) = claim_decision {
                self.follow_decision(
                    &imported.key,
                    decision,
                    Some(imported.command.clone()),
                    None,
                    &mut claims,
                );
            }
            rows.push(ManagerRow {
                id: imported.key.clone(),
                kind: ManagerRowKind::Import,
                means: if imported.key == tode_core::quit_chord(cfg!(target_os = "macos")) {
                    "quit terminal-code".into()
                } else {
                    words(&imported.builtin)
                },
                detail: None,
                terminal: ManagerTerminal {
                    name: self.provider.name.into(),
                    short: String::new(),
                    does: String::new(),
                    freed: String::new(),
                    tradeoff: String::new(),
                    bound: true,
                },
                imported_command: Some(imported.command.clone()),
                claimant: Some(imported.claimant.clone()),
                claim_describes: imported.describes.clone(),
                claim_decision: claim_decision.cloned(),
                decision: decision.cloned(),
                claims: deduplicate_claims(claims),
            });
        }
        let all_rows = rows.clone();
        rows.retain(|row| {
            if row.kind == ManagerRowKind::Terminal {
                if self.staged.contains_key(&row.id) {
                    return true;
                }
                let Some(current) = currents.get(&row.id) else {
                    return true;
                };
                let mirror = self.staged.get(&format!("claim:{}:{current}", row.id));
                if mirror.is_none_or(|decision| decision.choice != DecisionChoice::Terminal) {
                    return true;
                }
                !visible_elsewhere(&all_rows, row, current)
            } else {
                if self.staged.contains_key(&format!("import:{}", row.id)) {
                    return true;
                }
                if row
                    .claim_decision
                    .as_ref()
                    .is_none_or(|decision| decision.choice != DecisionChoice::Terminal)
                {
                    return true;
                }
                row.imported_command
                    .as_ref()
                    .is_none_or(|command| !visible_elsewhere(&all_rows, row, command))
            }
        });
        rows
    }

    pub fn taken(
        &mut self,
        chord: &str,
        row_id: Option<&str>,
        command: Option<&str>,
        side: Option<ManagerRowKind>,
    ) -> Option<TakenResult> {
        self.refresh();
        let normalized = normalize_chord(chord)?;
        let chord = normalized.as_str();
        let conflict = row_id.and_then(|id| {
            self.scan
                .terminal
                .iter()
                .find(|conflict| conflict.editor_id == id)
        });
        let own_keys: Vec<_> = command.map_or_else(
            || {
                vec![
                    row_id.unwrap_or_default().to_owned(),
                    format!("import:{}", row_id.unwrap_or_default()),
                    format!(
                        "claim:{}:{}",
                        row_id.unwrap_or_default(),
                        conflict
                            .and_then(|value| value.current.as_deref())
                            .unwrap_or_default()
                    ),
                ]
            },
            |command| {
                vec![
                    format!("claim:{}:{command}", row_id.unwrap_or_default()),
                    format!("claim:{}", row_id.unwrap_or_default()),
                ]
            },
        );
        for (id, decision) in &self.staged {
            if decision.key.as_deref() != Some(chord) || decision.choice == DecisionChoice::Keep {
                continue;
            }
            if row_id.is_some() && own_keys.contains(id) {
                continue;
            }
            let what = decision
                .action
                .as_deref()
                .or(decision.command.as_deref())
                .unwrap_or("another shortcut");
            return Some(TakenResult {
                holder: format!("{what} (moved here this session)"),
                claim: None,
            });
        }
        let own_action = command
            .filter(|command| {
                self.scan
                    .occupied
                    .get(row_id.unwrap_or_default())
                    .is_some_and(|value| value == command)
            })
            .map(str::to_owned)
            .or_else(|| {
                (side == Some(ManagerRowKind::Terminal))
                    .then(|| conflict.and_then(|value| value.current.clone()))
                    .flatten()
            });
        let own_command = if command.is_some() && own_action.is_none() {
            command.map(str::to_owned)
        } else if side == Some(ManagerRowKind::Import) {
            conflict
                .map(|value| value.editor.command.clone())
                .or_else(|| {
                    generated_bindings(None)
                        .into_iter()
                        .find(|binding| canonical_chord(&binding.key) == row_id.unwrap_or_default())
                        .map(|binding| binding.command)
                })
        } else {
            None
        };
        if let Some(action) = self.scan.occupied.get(chord)
            && own_action.as_ref() != Some(action)
        {
            let claim = ManagerClaim {
                chord: chord.into(),
                command: action.clone(),
                claimant: self.provider.name.into(),
                describes: words(action),
                when: None,
                resting: None,
                decided: self.staged_about(chord, action, true),
            };
            if claim
                .decided
                .as_ref()
                .is_none_or(|decision| decision.choice != DecisionChoice::Terminal)
            {
                return Some(TakenResult {
                    holder: format!("{} ({})", claim.command, claim.claimant),
                    claim: Some(claim),
                });
            }
        }
        for holder in self.holders.get(chord).into_iter().flatten() {
            if own_command.as_ref() == Some(&holder.command) {
                continue;
            }
            let claim = ManagerClaim {
                chord: chord.into(),
                command: holder.command.clone(),
                claimant: holder.claimant.clone(),
                describes: holder
                    .describes
                    .clone()
                    .unwrap_or_else(|| words(&holder.command)),
                when: holder.guard.clone(),
                resting: None,
                decided: self.staged_about(chord, &holder.command, false),
            };
            if claim
                .decided
                .as_ref()
                .is_some_and(|decision| decision.choice == DecisionChoice::Terminal)
            {
                continue;
            }
            return Some(TakenResult {
                holder: format!("{} ({})", claim.command, claim.claimant),
                claim: Some(claim),
            });
        }
        None
    }

    pub fn decide(
        &mut self,
        id: &str,
        kind: DecisionKind,
        mut decision: Option<Decision>,
        side_claim: bool,
        info: Option<ClaimInfo>,
    ) {
        self.refresh();
        let claim = kind == DecisionKind::Claim || side_claim;
        let key = if claim {
            info.as_ref().map_or_else(
                || format!("claim:{id}"),
                |info| format!("claim:{id}:{}", info.command),
            )
        } else if kind == DecisionKind::Import {
            format!("import:{id}")
        } else {
            id.to_owned()
        };
        self.drop_twin(id, kind, claim, info.as_ref(), &key);
        let Some(mut decision) = decision.take() else {
            self.staged.remove(&key);
            self.drop_twin(id, kind, claim, info.as_ref(), &key);
            return;
        };
        if let Some(normalized) = decision.key.as_deref().and_then(normalize_chord) {
            decision.key = Some(normalized);
        }
        if kind == DecisionKind::Import && decision.choice == DecisionChoice::Editor {
            decision.command = generated_bindings(None)
                .into_iter()
                .find(|binding| canonical_chord(&binding.key) == id)
                .map(|binding| binding.command)
                .or_else(|| {
                    self.staged
                        .get(&key)
                        .and_then(|value| value.command.clone())
                });
        }
        if kind == DecisionKind::Terminal {
            let conflict = self
                .scan
                .terminal
                .iter()
                .find(|conflict| conflict.editor_id == id);
            if decision.choice == DecisionChoice::Terminal {
                decision.action = conflict
                    .and_then(|conflict| conflict.current.clone())
                    .or_else(|| self.staged.get(&key).and_then(|value| value.action.clone()));
            }
            if decision.choice == DecisionChoice::Editor {
                decision.command = conflict
                    .map(|conflict| conflict.editor.command.clone())
                    .or_else(|| {
                        self.staged
                            .get(&key)
                            .and_then(|value| value.command.clone())
                    });
            }
        }
        if claim {
            let terminal = self.scan.occupied.get(id);
            if let Some(info) = &info {
                decision.action = Some(info.command.clone());
                decision.owner_terminal = terminal == Some(&info.command);
                if !decision.owner_terminal {
                    decision.guard = info.when.clone();
                }
            } else if let Some(action) = terminal {
                decision.action = Some(action.clone());
                decision.owner_terminal = true;
            } else if let Some(holder) = self.holders.get(id).and_then(|values| values.first()) {
                decision.action = Some(holder.command.clone());
                decision.guard = holder.guard.clone();
                decision.owner_terminal = false;
            }
        }
        self.staged.insert(key, decision);
    }

    pub fn confirm(&mut self) -> Result<bool, ShortcutError> {
        self.refresh();
        let decisions = Decisions {
            version: 1,
            terminal: terminal_id(&self.provider).into(),
            choices: self.staged.clone(),
        };
        apply_decisions(&self.provider, &self.paths, &self.scan.terminal, &decisions)
    }

    fn refresh(&mut self) {
        if let Ok(scan) = scan_shortcuts(&self.provider, &self.paths) {
            self.scan = scan;
            self.holders = editor_holders(&self.paths);
        }
    }

    fn drop_twin(
        &mut self,
        id: &str,
        kind: DecisionKind,
        claim: bool,
        info: Option<&ClaimInfo>,
        key: &str,
    ) {
        if kind == DecisionKind::Terminal {
            let current = self
                .scan
                .terminal
                .iter()
                .find(|conflict| conflict.editor_id == id)
                .and_then(|conflict| conflict.current.as_ref());
            if let Some(current) = current {
                self.staged.remove(&format!("claim:{id}:{current}"));
                if self
                    .staged
                    .get(&format!("claim:{id}"))
                    .is_some_and(|legacy| {
                        legacy.owner_terminal && legacy.action.as_ref() == Some(current)
                    })
                {
                    self.staged.remove(&format!("claim:{id}"));
                }
            }
        }
        if claim {
            let action = info.map(|value| value.command.as_str()).or_else(|| {
                self.staged
                    .get(key)
                    .and_then(|value| value.action.as_deref())
            });
            if self.staged.get(id).is_some_and(|twin| {
                twin.choice == DecisionChoice::Terminal && twin.action.as_deref() == action
            }) {
                self.staged.remove(id);
            }
        }
    }

    fn staged_about(&self, chord: &str, command: &str, terminal: bool) -> Option<Decision> {
        let claim = self
            .staged
            .get(&format!("claim:{chord}:{command}"))
            .or_else(|| self.staged.get(&format!("claim:{chord}")));
        if let Some(claim) = claim
            && claim.action.as_deref().unwrap_or(command) == command
        {
            return Some(claim.clone());
        }
        let row = self.staged.get(chord);
        if terminal {
            return row
                .filter(|decision| {
                    decision.choice == DecisionChoice::Terminal
                        && decision.action.as_deref() == Some(command)
                })
                .cloned();
        }
        let translate = |decision: Option<&Decision>| {
            let decision =
                decision.filter(|decision| decision.command.as_deref() == Some(command))?;
            if decision.choice == DecisionChoice::Keep {
                return Some(decision_for_claim(None));
            }
            (decision.choice == DecisionChoice::Editor
                && decision.key.as_deref().is_some_and(|key| key != chord))
            .then(|| decision_for_claim(decision.key.clone()))
        };
        translate(row).or_else(|| translate(self.staged.get(&format!("import:{chord}"))))
    }

    fn follow_decision(
        &self,
        row_chord: &str,
        decision: &Decision,
        exempt_command: Option<String>,
        exempt_action: Option<String>,
        output: &mut Vec<ManagerClaim>,
    ) {
        let mut seen = BTreeSet::new();
        self.follow_decision_inner(
            row_chord,
            decision,
            exempt_command,
            exempt_action,
            output,
            &mut seen,
        );
    }

    fn follow_decision_inner(
        &self,
        row_chord: &str,
        decision: &Decision,
        exempt_command: Option<String>,
        exempt_action: Option<String>,
        output: &mut Vec<ManagerClaim>,
        seen: &mut BTreeSet<(String, String)>,
    ) {
        let Some(target) = decision.key.as_deref() else {
            return;
        };
        if decision.choice == DecisionChoice::Keep || target == row_chord {
            return;
        }
        let mut queue = VecDeque::new();
        if let Some(action) = self.scan.occupied.get(target)
            && exempt_action.as_ref() != Some(action)
        {
            let decided = self.staged_about(target, action, true);
            queue.push_back(PendingClaim {
                claim: ManagerClaim {
                    chord: target.into(),
                    command: action.clone(),
                    claimant: self.provider.name.into(),
                    describes: words(action),
                    when: None,
                    resting: None,
                    decided: decided.clone(),
                },
                decision: decided,
                exempt_command: None,
                exempt_action: Some(action.clone()),
            });
        }
        for holder in self.holders.get(target).into_iter().flatten() {
            if exempt_command.as_ref() == Some(&holder.command) {
                continue;
            }
            let decided = self.staged_about(target, &holder.command, false);
            queue.push_back(PendingClaim {
                claim: ManagerClaim {
                    chord: target.into(),
                    command: holder.command.clone(),
                    claimant: holder.claimant.clone(),
                    describes: holder
                        .describes
                        .clone()
                        .unwrap_or_else(|| words(&holder.command)),
                    when: holder.guard.clone(),
                    resting: None,
                    decided: decided.clone(),
                },
                decision: decided,
                exempt_command: Some(holder.command.clone()),
                exempt_action: None,
            });
        }
        while let Some(pending) = queue.pop_front() {
            if !seen.insert((pending.claim.chord.clone(), pending.claim.command.clone())) {
                continue;
            }
            output.push(pending.claim.clone());
            if let Some(decision) = pending.decision {
                self.follow_decision_inner(
                    row_chord,
                    &decision,
                    pending.exempt_command,
                    pending.exempt_action,
                    output,
                    seen,
                );
            }
        }
    }
}

fn decision_for_claim(key: Option<String>) -> Decision {
    Decision {
        choice: DecisionChoice::Terminal,
        key,
        action: None,
        guard: None,
        owner_terminal: false,
        command: None,
    }
}

fn terminal_row(provider: &TerminalProvider, conflict: &ShortcutConflict) -> ManagerTerminal {
    ManagerTerminal {
        name: provider.name.into(),
        short: conflict.short.clone(),
        does: conflict.in_terminal.clone(),
        freed: conflict.freed.clone(),
        tradeoff: conflict.tradeoff.clone(),
        bound: true,
    }
}

fn deduplicate_claims(claims: Vec<ManagerClaim>) -> Vec<ManagerClaim> {
    let mut seen = BTreeSet::new();
    claims
        .into_iter()
        .filter(|claim| seen.insert((claim.chord.clone(), claim.command.clone())))
        .collect()
}

fn visible_elsewhere(rows: &[ManagerRow], row: &ManagerRow, command: &str) -> bool {
    rows.iter().any(|other| {
        other.id != row.id
            && other
                .claims
                .iter()
                .any(|claim| claim.chord == row.id && claim.command == command)
    })
}

fn terminal_id(provider: &TerminalProvider) -> &'static str {
    match provider.kind {
        crate::shortcuts::TerminalKind::Ghostty => "ghostty",
        crate::shortcuts::TerminalKind::Kitty => "kitty",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use tempfile::TempDir;
    use tode_core::Binding;

    use super::*;
    use crate::shortcuts::{detect_provider, undo_shortcuts};

    fn fixture() -> (TempDir, BTreeMap<OsString, OsString>, ProfilePaths) {
        let root = TempDir::new().unwrap();
        let bin = root.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let ghostty = bin.join("ghostty");
        fs::write(
            &ghostty,
            "#!/bin/sh\nprintf 'keybind = ctrl+p=new_tab\\nkeybind = ctrl+k=close_tab\\n'\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&ghostty).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&ghostty, permissions).unwrap();
        let environment = BTreeMap::from([
            (OsString::from("PATH"), bin.into_os_string()),
            (OsString::from("TERM_PROGRAM"), OsString::from("ghostty")),
            (
                OsString::from("XDG_DATA_HOME"),
                root.path().join("data").into_os_string(),
            ),
            (
                OsString::from("XDG_CONFIG_HOME"),
                root.path().join("config").into_os_string(),
            ),
        ]);
        let paths = ProfilePaths::from_environment(root.path(), &environment);
        fs::create_dir_all(&paths.user).unwrap();
        fs::write(
            paths.user.join("keybindings.json"),
            r#"[{"key":"ctrl+p","command":"workbench.action.quickOpen"},{"key":"ctrl+k","command":"mine.close"},{"key":"ctrl+u","command":"mine.other"}]"#,
        )
        .unwrap();
        (root, environment, paths)
    }

    fn decision(choice: DecisionChoice, key: Option<&str>) -> Decision {
        Decision {
            choice,
            key: key.map(str::to_owned),
            action: None,
            guard: None,
            owner_terminal: false,
            command: None,
        }
    }

    #[test]
    fn rows_and_taken_track_staged_moves() {
        let (root, environment, paths) = fixture();
        let provider = detect_provider(root.path(), &environment).unwrap();
        let mut session = ShortcutSession::new(provider, paths).unwrap();
        assert_eq!(session.rows().len(), 2);
        assert!(
            session
                .taken(
                    "ctrl+k",
                    Some("ctrl+p"),
                    None,
                    Some(ManagerRowKind::Terminal)
                )
                .is_some()
        );
        session.decide(
            "ctrl+p",
            DecisionKind::Terminal,
            Some(decision(DecisionChoice::Terminal, Some("ctrl+u"))),
            false,
            None,
        );
        assert_eq!(session.staged["ctrl+p"].action.as_deref(), Some("new_tab"));
        assert!(
            session
                .taken(
                    "ctrl+u",
                    Some("ctrl+k"),
                    None,
                    Some(ManagerRowKind::Terminal)
                )
                .is_some()
        );
    }

    #[test]
    fn confirm_persists_and_applies_then_reopens_cleanly() {
        let (root, environment, paths) = fixture();
        let provider = detect_provider(root.path(), &environment).unwrap();
        let mut session = ShortcutSession::new(provider, paths.clone()).unwrap();
        for id in ["ctrl+p", "ctrl+k"] {
            session.decide(
                id,
                DecisionKind::Terminal,
                Some(decision(DecisionChoice::Terminal, None)),
                false,
                None,
            );
        }
        assert!(session.confirm().unwrap());
        assert!(paths.data.join("shortcuts.json").is_file());
        let config = fs::read_to_string(root.path().join(if cfg!(target_os = "macos") {
            "Library/Application Support/com.mitchellh.ghostty/tode/keybinds.ghostty"
        } else {
            ".config/ghostty/tode/keybinds.ghostty"
        }))
        .unwrap_or_default();
        assert!(config.contains("ctrl+p"));
        let provider = detect_provider(root.path(), &environment).unwrap();
        let reopened = ShortcutSession::new(provider, paths).unwrap();
        assert_eq!(reopened.staged.len(), 2);
    }

    #[test]
    fn decision_twin_is_not_staged_twice() {
        let (root, environment, paths) = fixture();
        let provider = detect_provider(root.path(), &environment).unwrap();
        let mut session = ShortcutSession::new(provider, paths).unwrap();
        session.decide(
            "ctrl+p",
            DecisionKind::Claim,
            Some(decision(DecisionChoice::Terminal, None)),
            true,
            Some(ClaimInfo {
                command: "new_tab".into(),
                when: None,
            }),
        );
        session.decide(
            "ctrl+p",
            DecisionKind::Terminal,
            Some(decision(DecisionChoice::Terminal, None)),
            false,
            None,
        );
        assert!(session.staged.contains_key("ctrl+p"));
        assert!(!session.staged.contains_key("claim:ctrl+p:new_tab"));
    }
    #[test]
    fn cyclic_claim_moves_terminate_and_deduplicate() {
        let (root, environment, paths) = fixture();
        let provider = detect_provider(root.path(), &environment).unwrap();
        let mut session = ShortcutSession::new(provider, paths).unwrap();
        session.staged.insert(
            "ctrl+p".into(),
            decision(DecisionChoice::Editor, Some("ctrl+k")),
        );
        session.staged.insert(
            "claim:ctrl+k:mine.close".into(),
            decision(DecisionChoice::Terminal, Some("ctrl+u")),
        );
        session.staged.insert(
            "claim:ctrl+u:mine.other".into(),
            decision(DecisionChoice::Terminal, Some("ctrl+k")),
        );
        let row = session
            .rows()
            .into_iter()
            .find(|row| row.id == "ctrl+p")
            .unwrap();
        let identities: BTreeSet<_> = row
            .claims
            .iter()
            .map(|claim| (&claim.chord, &claim.command))
            .collect();
        assert_eq!(identities.len(), row.claims.len());
    }
    #[derive(Debug, Clone, Copy)]
    enum Strategy {
        Unset,
        Keep,
        EditorMove,
        TerminalMove,
        ContestedMove,
    }

    fn loop_fixture() -> (TempDir, BTreeMap<OsString, OsString>, ProfilePaths, String) {
        let root = TempDir::new().unwrap();
        let bin = root.path().join("bin");
        let config = root.path().join("config/ghostty");
        let owned = config.join("tode/keybinds.ghostty");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(config.join("config"), "font-size = 14\n").unwrap();
        let ghostty = bin.join("ghostty");
        fs::write(
            &ghostty,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"+list-actions\" ]; then\n  printf 'new_tab:\\n  Opens a new tab.\\nclose_surface:\\n  Closes a surface.\\n'\nelse\n  printf 'keybind = super+w=close_surface\\nkeybind = super+digit_1=goto_tab:1\\nkeybind = super+f=start_search\\nkeybind = super+t=new_tab\\nkeybind = super+alt+shift+z=close_tab\\n'\n  if [ -f '{}' ]; then cat '{}'; fi\nfi\n",
                owned.display(),
                owned.display()
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&ghostty).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&ghostty, permissions).unwrap();
        let environment = BTreeMap::from([
            (OsString::from("PATH"), bin.into_os_string()),
            (OsString::from("TERM_PROGRAM"), OsString::from("ghostty")),
            (
                OsString::from("XDG_DATA_HOME"),
                root.path().join("data").into_os_string(),
            ),
            (
                OsString::from("XDG_STATE_HOME"),
                root.path().join("state").into_os_string(),
            ),
            (
                OsString::from("XDG_CONFIG_HOME"),
                root.path().join("config").into_os_string(),
            ),
        ]);
        let paths = ProfilePaths::from_environment(root.path(), &environment);
        fs::create_dir_all(&paths.user).unwrap();
        fs::create_dir_all(&paths.extensions).unwrap();
        let imported = serde_json::to_string(&vec![
            Binding {
                key: tode_core::quit_chord(cfg!(target_os = "macos")).into(),
                command: "workbench.action.terminal.kill".into(),
                when: None,
            },
            Binding {
                key: "alt+w".into(),
                command: "my.imported.closeThing".into(),
                when: None,
            },
            Binding {
                key: "cmd+1".into(),
                command: "workbench.action.focusFirstEditorGroup".into(),
                when: None,
            },
            Binding {
                key: "cmd+w".into(),
                command: "workbench.action.closeWindow".into(),
                when: None,
            },
            Binding {
                key: "cmd+f".into(),
                command: "actions.find".into(),
                when: None,
            },
            Binding {
                key: "cmd+t".into(),
                command: "workbench.action.newWindow".into(),
                when: None,
            },
        ])
        .unwrap();
        fs::write(paths.user.join("keybindings.json"), &imported).unwrap();
        let vim = paths.extensions.join("vscodevim.vim-1.30.0");
        fs::create_dir(&vim).unwrap();
        fs::write(
            vim.join("package.json"),
            r#"{"name":"vim","displayName":"Vim","contributes":{"keybindings":[{"key":"ctrl+d","mac":"cmd+d","command":"extension.vim_ctrl+d"},{"key":"alt+3","command":"extension.vim_window3","when":"vim.active"}]}}"#,
        )
        .unwrap();
        let acme = paths.extensions.join("acme.keys-1.0.0");
        fs::create_dir(&acme).unwrap();
        fs::write(
            acme.join("package.json"),
            r#"{"name":"acme-keys","displayName":"Acme Keys","contributes":{"keybindings":[{"key":"ctrl+alt+k","command":"acme.everything"}]}}"#,
        )
        .unwrap();
        (root, environment, paths, imported)
    }

    fn candidates() -> Vec<String> {
        ["alt", "ctrl+alt", "shift+alt", "cmd+alt"]
            .into_iter()
            .flat_map(|modifiers| {
                "abcdefghijklmnopqrstuvwxyz0123456789"
                    .chars()
                    .map(move |key| format!("{modifiers}+{key}"))
            })
            .collect()
    }

    fn row_chords(row: &ManagerRow) -> Vec<String> {
        let mut chords = Vec::new();
        let moved = if row.kind == ManagerRowKind::Import {
            row.claim_decision
                .as_ref()
                .filter(|decision| decision.choice == DecisionChoice::Terminal)
        } else {
            row.decision
                .as_ref()
                .filter(|decision| decision.choice == DecisionChoice::Terminal)
        };
        if let Some(left) = moved.map_or(Some(row.id.as_str()), |decision| decision.key.as_deref())
        {
            chords.push(left.to_owned());
        }
        if row
            .decision
            .as_ref()
            .is_none_or(|decision| decision.choice != DecisionChoice::Keep)
        {
            chords.push(
                row.decision
                    .as_ref()
                    .filter(|decision| decision.choice == DecisionChoice::Editor)
                    .and_then(|decision| decision.key.clone())
                    .unwrap_or_else(|| row.id.clone()),
            );
        }
        for claim in &row.claims {
            if claim.resting == Some(true) && claim.decided.is_none() {
                continue;
            }
            if let Some(chord) = claim
                .decided
                .as_ref()
                .map_or(Some(claim.chord.as_str()), |decision| {
                    decision.key.as_deref()
                })
            {
                chords.push(chord.to_owned());
            }
        }
        chords
    }

    fn unresolved(row: &ManagerRow) -> bool {
        let mut counts = BTreeMap::new();
        for chord in row_chords(row) {
            *counts.entry(chord).or_insert(0_u8) += 1;
        }
        counts.values().any(|count| *count > 1)
            || row
                .claims
                .iter()
                .any(|claim| claim.resting != Some(true) && claim.decided.is_none())
    }

    struct PickRequest<'a> {
        row_id: &'a str,
        command: Option<&'a str>,
        side: Option<ManagerRowKind>,
        contested: bool,
    }

    fn pick(
        session: &mut ShortcutSession,
        candidates: &[String],
        offset: usize,
        request: PickRequest<'_>,
        contested_picks: &mut usize,
    ) -> String {
        for at in 0..candidates.len() {
            let chord = &candidates[(offset + at) % candidates.len()];
            match session.taken(chord, Some(request.row_id), request.command, request.side) {
                None => return chord.clone(),
                Some(taken) if request.contested && taken.claim.is_some() => {
                    *contested_picks += 1;
                    return chord.clone();
                }
                Some(_) => {}
            }
        }
        panic!("no move target for {}", request.row_id);
    }

    fn resolve_world(
        session: &mut ShortcutSession,
        strategy: Strategy,
        offset: usize,
        contested_picks: &mut usize,
    ) {
        let candidates = candidates();
        for _ in 0..400 {
            let Some(row) = session.rows().into_iter().find(unresolved) else {
                return;
            };
            if let Some(claim) = row
                .claims
                .iter()
                .find(|claim| claim.resting != Some(true) && claim.decided.is_none())
                .cloned()
            {
                let key = matches!(strategy, Strategy::TerminalMove).then(|| {
                    pick(
                        session,
                        &candidates,
                        offset,
                        PickRequest {
                            row_id: &claim.chord,
                            command: Some(&claim.command),
                            side: None,
                            contested: false,
                        },
                        contested_picks,
                    )
                });
                let mut choice = decision(DecisionChoice::Terminal, key.as_deref());
                choice.action = Some(claim.command.clone());
                session.decide(
                    &claim.chord,
                    DecisionKind::Claim,
                    Some(choice),
                    false,
                    Some(ClaimInfo {
                        command: claim.command,
                        when: claim.when,
                    }),
                );
                continue;
            }
            if row.kind == ManagerRowKind::Import {
                let key =
                    matches!(strategy, Strategy::TerminalMove | Strategy::EditorMove).then(|| {
                        pick(
                            session,
                            &candidates,
                            offset,
                            PickRequest {
                                row_id: &row.id,
                                command: row.imported_command.as_deref(),
                                side: None,
                                contested: false,
                            },
                            contested_picks,
                        )
                    });
                session.decide(
                    &row.id,
                    DecisionKind::Import,
                    Some(decision(DecisionChoice::Terminal, key.as_deref())),
                    true,
                    row.imported_command.map(|command| ClaimInfo {
                        command,
                        when: None,
                    }),
                );
                continue;
            }
            let choice = match strategy {
                Strategy::Keep => decision(DecisionChoice::Keep, None),
                Strategy::EditorMove => {
                    let key = pick(
                        session,
                        &candidates,
                        offset,
                        PickRequest {
                            row_id: &row.id,
                            command: None,
                            side: Some(ManagerRowKind::Import),
                            contested: false,
                        },
                        contested_picks,
                    );
                    decision(DecisionChoice::Editor, Some(&key))
                }
                Strategy::TerminalMove | Strategy::ContestedMove => {
                    let key = pick(
                        session,
                        &candidates,
                        offset,
                        PickRequest {
                            row_id: &row.id,
                            command: None,
                            side: Some(ManagerRowKind::Terminal),
                            contested: matches!(strategy, Strategy::ContestedMove),
                        },
                        contested_picks,
                    );
                    decision(DecisionChoice::Terminal, Some(&key))
                }
                Strategy::Unset => decision(DecisionChoice::Terminal, None),
            };
            session.decide(&row.id, DecisionKind::Terminal, Some(choice), false, None);
        }
        panic!("shortcut manager did not converge");
    }

    fn snapshot(paths: &ProfilePaths, owned: &Path) -> Vec<u8> {
        [
            fs::read(owned).unwrap_or_default(),
            fs::read(paths.user.join("keybindings.json")).unwrap_or_default(),
        ]
        .concat()
    }

    #[test]
    fn adversarial_closed_loop_reopens_clean_and_byte_idempotent() {
        let (root, environment, paths, imported) = loop_fixture();
        let provider = detect_provider(root.path(), &environment).unwrap();
        let owned = provider.config_dir.join("tode/keybinds.ghostty");
        let mut contested_picks = 0;
        let candidate_count = candidates().len();
        let mut runs = vec![
            (Strategy::Unset, 0),
            (Strategy::Keep, 0),
            (Strategy::EditorMove, 0),
        ];
        runs.extend(
            (0..candidate_count)
                .step_by(3)
                .map(|offset| (Strategy::TerminalMove, offset)),
        );
        runs.extend(
            (0..candidate_count)
                .step_by(5)
                .map(|offset| (Strategy::ContestedMove, offset)),
        );
        for (strategy, offset) in runs {
            let mut session = ShortcutSession::new(provider.clone(), paths.clone()).unwrap();
            assert!(session.rows().iter().any(unresolved));
            resolve_world(&mut session, strategy, offset, &mut contested_picks);
            session.confirm().unwrap();
            let mut reopened = ShortcutSession::new(provider.clone(), paths.clone()).unwrap();
            let unresolved_rows: Vec<_> = reopened.rows().into_iter().filter(unresolved).collect();
            assert!(
                unresolved_rows.is_empty(),
                "{strategy:?}@{offset}: {unresolved_rows:#?}\ndecisions={:#?}\nkeybindings={}",
                crate::shortcuts::load_decisions(&paths),
                fs::read_to_string(paths.user.join("keybindings.json")).unwrap_or_default(),
            );
            let before = snapshot(&paths, &owned);
            reopened.confirm().unwrap();
            assert_eq!(snapshot(&paths, &owned), before, "{strategy:?}");
            undo_shortcuts(&provider, &paths).unwrap();
            fs::write(paths.user.join("keybindings.json"), &imported).unwrap();
        }
        assert!(contested_picks > 0);

        let mut session = ShortcutSession::new(provider, paths.clone()).unwrap();
        let late_chord = canonical_chord("cmd+alt+shift+z");
        assert!(!session.rows().iter().any(|row| row.id == late_chord));
        let mut late: Vec<Binding> = serde_json::from_str(&imported).unwrap();
        late.push(Binding {
            key: "cmd+alt+shift+z".into(),
            command: "my.late.import".into(),
            when: None,
        });
        fs::write(
            paths.user.join("keybindings.json"),
            serde_json::to_vec(&late).unwrap(),
        )
        .unwrap();
        let late_extension = paths.extensions.join("late.ext-1.0.0");
        fs::create_dir(&late_extension).unwrap();
        fs::write(
            late_extension.join("package.json"),
            r#"{"name":"late","displayName":"Late","contributes":{"keybindings":[{"key":"cmd+alt+shift+z","command":"late.thing"}]}}"#,
        )
        .unwrap();
        let late_row = session
            .rows()
            .into_iter()
            .find(|row| row.id == late_chord)
            .unwrap();
        assert!(late_row.claims.iter().any(|claim| claim.claimant == "Late"));
    }
}
