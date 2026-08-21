use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MODS: [&str; 4] = ["ctrl", "shift", "alt", "cmd"];
pub const GHOSTTY_INCLUDE_LINE: &str = "config-file = ?tode/keybinds.ghostty";
pub const KITTY_INCLUDE_LINE: &str = "include tode/keybinds.kitty.conf";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreedMove {
    pub trigger: String,
    pub to: Option<String>,
    pub action: Option<String>,
    pub emit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionChoice {
    Terminal,
    Editor,
    Keep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub choice: DecisionChoice,
    pub key: Option<String>,
    pub action: Option<String>,
    pub guard: Option<String>,
    pub owner_terminal: bool,
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decisions {
    pub version: u8,
    pub terminal: String,
    pub choices: BTreeMap<String, Decision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub key: String,
    pub command: String,
    pub when: Option<String>,
}

pub fn quit_chord(is_macos: bool) -> &'static str {
    if is_macos { "ctrl+c" } else { "ctrl+q" }
}

pub fn claim_bindings(decisions: &Decisions) -> Vec<Binding> {
    let mut output = Vec::new();
    for (id, decision) in &decisions.choices {
        if !id.starts_with("claim:")
            || decision.owner_terminal
            || decision.choice != DecisionChoice::Terminal
        {
            continue;
        }
        let Some(action) = &decision.action else {
            continue;
        };
        let rest = &id["claim:".len()..];
        let chord = rest.split_once(':').map_or(rest, |(chord, _)| chord);
        output.push(Binding {
            key: chord.into(),
            command: format!("-{action}"),
            when: None,
        });
        if let Some(key) = &decision.key {
            output.push(Binding {
                key: key.clone(),
                command: action.clone(),
                when: decision.guard.clone(),
            });
        }
    }
    output
}

pub fn override_bindings(decisions: &Decisions, is_macos: bool) -> Vec<Binding> {
    let import_quit = format!("import:{}", quit_chord(is_macos));
    decisions
        .choices
        .iter()
        .filter_map(|(id, decision)| {
            if !id.starts_with("import:")
                || decision.choice != DecisionChoice::Editor
                || decision.key.is_none()
            {
                return None;
            }
            let command = decision
                .command
                .clone()
                .or_else(|| (id == &import_quit).then(|| "tode.confirmQuit".into()))?;
            Some(Binding {
                key: decision.key.clone().unwrap(),
                command,
                when: Some("!terminalFocus".into()),
            })
        })
        .collect()
}

pub fn quit_when(is_macos: bool) -> &'static str {
    if is_macos {
        "!terminalFocus && !editorHasSelection && (!inputFocus || editorTextFocus)"
    } else {
        "!terminalFocus"
    }
}

pub fn hint_bindings(is_macos: bool) -> Vec<Binding> {
    if is_macos {
        Vec::new()
    } else {
        vec![Binding {
            key: "ctrl+c".into(),
            command: "tode.quitHint".into(),
            when: Some(
                "!terminalFocus && !editorHasSelection && (!inputFocus || editorTextFocus)".into(),
            ),
        }]
    }
}

pub fn quit_bindings(decisions: Option<&Decisions>, is_macos: bool) -> Vec<Binding> {
    let chord = quit_chord(is_macos);
    let import = format!("import:{chord}");
    let decision = decisions.and_then(|decisions| {
        decisions
            .choices
            .get(&import)
            .or_else(|| decisions.choices.get(chord))
    });
    if matches!(
        decision.map(|decision| decision.choice),
        Some(DecisionChoice::Editor | DecisionChoice::Keep)
    ) {
        return Vec::new();
    }
    vec![Binding {
        key: chord.into(),
        command: "tode.confirmQuit".into(),
        when: Some(quit_when(is_macos).into()),
    }]
}

pub fn fallback_bindings(decisions: Option<&Decisions>) -> Vec<Binding> {
    let Some(decisions) = decisions else {
        return Vec::new();
    };
    decisions
        .choices
        .iter()
        .filter_map(|(id, decision)| {
            if id.starts_with("claim:")
                || id.starts_with("import:")
                || decision.choice != DecisionChoice::Editor
            {
                return None;
            }
            Some(Binding {
                key: decision.key.clone()?,
                command: decision.command.clone()?,
                when: Some("!terminalFocus".into()),
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGhosttyTrigger {
    pub trigger: String,
    pub passes_through: bool,
}

pub fn canonical_chord(chord: &str) -> String {
    let opener = chord.split_whitespace().next().unwrap_or("");
    let mut parts: Vec<_> = opener
        .to_lowercase()
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect();
    let Some(key) = parts.pop() else {
        return String::new();
    };
    let normalized: Vec<_> = parts
        .into_iter()
        .map(|part| {
            if matches!(part.as_str(), "meta" | "super") {
                "cmd".into()
            } else {
                part
            }
        })
        .collect();
    let mut out: Vec<String> = MODS
        .iter()
        .filter(|modifier| normalized.iter().any(|part| part == **modifier))
        .map(|modifier| (*modifier).to_owned())
        .collect();
    out.push(key);
    out.join("+")
}

pub fn normalize_chord(input: &str) -> Option<String> {
    let mut parts: Vec<_> = input
        .to_lowercase()
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect();
    if parts.len() < 2 {
        return None;
    }
    let key = parts.pop()?;
    let mut modifiers = Vec::new();
    for part in parts {
        let modifier = match part.as_str() {
            "meta" | "super" | "command" => "cmd".to_owned(),
            "control" => "ctrl".to_owned(),
            "option" | "opt" => "alt".to_owned(),
            value => value.to_owned(),
        };
        if !MODS.contains(&modifier.as_str()) {
            return None;
        }
        if !modifiers.contains(&modifier) {
            modifiers.push(modifier);
        }
    }
    if !valid_key(&key) {
        return None;
    }
    let mut out: Vec<_> = MODS
        .iter()
        .filter(|modifier| modifiers.iter().any(|value| value == **modifier))
        .copied()
        .collect();
    out.push(&key);
    Some(out.join("+"))
}

pub fn ghostty_to_trigger(chord: &str) -> String {
    chord
        .split('+')
        .map(|part| match part {
            "cmd" => "super".into(),
            "left" | "right" | "up" | "down" => format!("arrow_{part}"),
            "pageup" => "page_up".into(),
            "pagedown" => "page_down".into(),
            "`" => "grave_accent".into(),
            value => value.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

pub fn parse_ghostty_trigger(raw: &str) -> ParsedGhosttyTrigger {
    let mut trigger = raw.trim();
    let mut passes_through = false;
    while let Some((prefix, rest)) = trigger.split_once(':') {
        if !matches!(prefix, "global" | "all" | "unconsumed" | "performable") {
            break;
        }
        if matches!(prefix, "unconsumed" | "performable") {
            passes_through = true;
        }
        trigger = rest;
    }
    ParsedGhosttyTrigger {
        trigger: trigger.to_owned(),
        passes_through,
    }
}

pub fn ghostty_from_trigger(trigger: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in trigger.split('+') {
        let mapped = match part {
            "super" => "cmd",
            "arrow_left" => "left",
            "arrow_right" => "right",
            "arrow_up" => "up",
            "arrow_down" => "down",
            "page_up" => "pageup",
            "page_down" => "pagedown",
            "grave_accent" => "`",
            "copy" | "paste" => return None,
            value if value.len() == 7 && value.starts_with("digit_") => &value[6..],
            value => value,
        };
        parts.push(mapped);
    }
    Some(canonical_chord(&parts.join("+")))
}

pub fn ghostty_keybinds_file(moves: &[FreedMove]) -> String {
    let mut lines = Vec::new();
    for movement in moves {
        lines.push(format!(
            "keybind = {}={}",
            movement.trigger,
            movement.emit.as_deref().unwrap_or("unbind")
        ));
        if let (Some(to), Some(action)) = (&movement.to, &movement.action) {
            lines.push(format!("keybind = {}={action}", ghostty_to_trigger(to)));
        }
    }
    format!(
        "# written by tode --shortcut-setup — frees the chords the editor needs from ghostty\n{}\n",
        lines.join("\n")
    )
}

pub fn ghostty_with_include(config: &str) -> String {
    with_include(config, GHOSTTY_INCLUDE_LINE)
}

pub fn ghostty_without_include(config: &str) -> String {
    without_include(config, GHOSTTY_INCLUDE_LINE)
}

pub fn emit_sequence(chord: &str) -> Option<String> {
    let mut parts: Vec<_> = chord.split('+').collect();
    let key = parts.pop()?;
    let codepoint = match key {
        "tab" => 9,
        "enter" => 13,
        "escape" => 27,
        "space" => 32,
        value if value.chars().count() == 1 => value.chars().next()? as u32,
        _ => return None,
    };
    let modifiers = 1
        + u32::from(parts.contains(&"shift"))
        + 2 * u32::from(parts.contains(&"alt"))
        + 4 * u32::from(parts.contains(&"ctrl"))
        + 8 * u32::from(parts.contains(&"cmd"));
    Some(format!("esc:[27;{modifiers};{codepoint}~"))
}

pub fn kitty_to_trigger(chord: &str) -> String {
    chord
        .split('+')
        .map(|part| match part {
            "cmd" => "super",
            "pageup" => "page_up",
            "pagedown" => "page_down",
            value => value,
        })
        .collect::<Vec<_>>()
        .join("+")
}

pub fn kitty_from_trigger(trigger: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in trigger.split('+') {
        let mapped = match part {
            "super" | "cmd" | "command" => "cmd",
            "opt" | "option" => "alt",
            "control" => "ctrl",
            "page_up" => "pageup",
            "page_down" => "pagedown",
            "plus" | "insert" | "menu" | "print_screen" => return None,
            value if value.starts_with("kp_") => return None,
            value => value,
        };
        parts.push(mapped);
    }
    Some(canonical_chord(&parts.join("+")))
}

pub fn kitty_with_shared_rebinds(moves: &[FreedMove]) -> Vec<FreedMove> {
    moves
        .iter()
        .map(|movement| {
            if movement.emit.is_some() || movement.to.is_some() {
                return movement.clone();
            }
            let first = movement
                .action
                .as_deref()
                .and_then(|action| action.split_whitespace().next());
            if first == Some("copy_to_clipboard") {
                let mut movement = movement.clone();
                movement.emit = Some("copy_or_noop".into());
                movement
            } else {
                movement.clone()
            }
        })
        .collect()
}

pub fn kitty_keybinds_file(moves: &[FreedMove]) -> String {
    let mut lines = Vec::new();
    for movement in moves {
        lines.push(match &movement.emit {
            Some(emit) => format!("map {} {emit}", movement.trigger),
            None => format!("map {}", movement.trigger),
        });
        if let (Some(to), Some(action)) = (&movement.to, &movement.action)
            && !action.starts_with("key sequence")
        {
            lines.push(format!("map {} {action}", kitty_to_trigger(to)));
        }
    }
    format!(
        "# written by tode --shortcut-setup — frees the chords the editor needs from kitty\n{}\n",
        lines.join("\n")
    )
}

pub fn kitty_with_include(config: &str) -> String {
    with_include(config, KITTY_INCLUDE_LINE)
}

pub fn kitty_without_include(config: &str) -> String {
    without_include(config, KITTY_INCLUDE_LINE)
}

fn with_include(config: &str, include: &str) -> String {
    if config.lines().any(|line| line.trim() == include) {
        return config.to_owned();
    }
    let separated = if !config.is_empty() && !config.ends_with('\n') {
        format!("{config}\n")
    } else {
        config.to_owned()
    };
    format!("{separated}{include}\n")
}

fn without_include(config: &str, include: &str) -> String {
    config
        .split('\n')
        .filter(|line| line.trim() != include)
        .collect::<Vec<_>>()
        .join("\n")
}

fn valid_key(key: &str) -> bool {
    if key.len() == 1 {
        return key.as_bytes()[0].is_ascii_alphanumeric() || "`-=[]\\;',./".contains(key);
    }
    if let Some(number) = key.strip_prefix('f') {
        return (1..=2).contains(&number.len()) && number.bytes().all(|byte| byte.is_ascii_digit());
    }
    matches!(
        key,
        "left"
            | "right"
            | "up"
            | "down"
            | "home"
            | "end"
            | "pageup"
            | "pagedown"
            | "tab"
            | "enter"
            | "space"
            | "backspace"
            | "delete"
            | "escape"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movement(trigger: &str, to: Option<&str>, action: Option<&str>) -> FreedMove {
        FreedMove {
            trigger: trigger.into(),
            to: to.map(str::to_owned),
            action: action.map(str::to_owned),
            emit: None,
        }
    }

    #[test]
    fn canonicalizes_aliases_sequences_and_modifier_order() {
        assert_eq!(canonical_chord("cmd+shift+p"), "shift+cmd+p");
        assert_eq!(canonical_chord("meta+F"), "cmd+f");
        assert_eq!(canonical_chord("ctrl+k ctrl+s"), "ctrl+k");
    }

    #[test]
    fn validates_and_normalizes_user_chords() {
        assert_eq!(normalize_chord("alt+ctrl+W").as_deref(), Some("ctrl+alt+w"));
        assert_eq!(
            normalize_chord("shift + control + t").as_deref(),
            Some("ctrl+shift+t")
        );
        assert_eq!(normalize_chord("cmd+left").as_deref(), Some("cmd+left"));
        assert_eq!(normalize_chord("w"), None);
        assert_eq!(normalize_chord("hyper+w"), None);
    }

    #[test]
    fn ghostty_trigger_conversion_is_invertible_for_supported_keys() {
        for chord in ["cmd+w", "cmd+left", "ctrl+pageup", "cmd+`", "shift+cmd+1"] {
            assert_eq!(
                ghostty_from_trigger(&ghostty_to_trigger(chord)).as_deref(),
                Some(canonical_chord(chord).as_str())
            );
        }
        assert_eq!(ghostty_from_trigger("super+copy"), None);
        assert_eq!(
            ghostty_from_trigger("super+digit_1").as_deref(),
            Some("cmd+1")
        );
    }

    #[test]
    fn ghostty_prefixes_report_pass_through() {
        assert_eq!(
            parse_ghostty_trigger("global:unconsumed:super+w"),
            ParsedGhosttyTrigger {
                trigger: "super+w".into(),
                passes_through: true
            }
        );
        assert!(!parse_ghostty_trigger("all:super+w").passes_through);
    }

    #[test]
    fn ghostty_file_moves_actions_and_include_is_idempotent() {
        let contents = ghostty_keybinds_file(&[movement(
            "super+w",
            Some("ctrl+alt+w"),
            Some("close_surface"),
        )]);
        assert!(contents.contains("keybind = super+w=unbind"));
        assert!(contents.contains("keybind = ctrl+alt+w=close_surface"));
        let config = ghostty_with_include("window-save-state = always");
        assert_eq!(config.matches(GHOSTTY_INCLUDE_LINE).count(), 1);
        assert_eq!(ghostty_with_include(&config), config);
        assert!(!ghostty_without_include(&config).contains(GHOSTTY_INCLUDE_LINE));
    }

    #[test]
    fn emit_sequences_match_terminal_protocol() {
        assert_eq!(emit_sequence("ctrl+tab").as_deref(), Some("esc:[27;5;9~"));
        assert_eq!(
            emit_sequence("ctrl+shift+tab").as_deref(),
            Some("esc:[27;6;9~")
        );
        assert_eq!(emit_sequence("cmd+space").as_deref(), Some("esc:[27;9;32~"));
        assert_eq!(emit_sequence("cmd+left"), None);
    }

    #[test]
    fn kitty_trigger_aliases_and_unsupported_keys_are_preserved() {
        assert_eq!(kitty_to_trigger("cmd+pageup"), "super+page_up");
        assert_eq!(
            kitty_from_trigger("command+option+f").as_deref(),
            Some("alt+cmd+f")
        );
        assert_eq!(
            kitty_from_trigger("control+page_down").as_deref(),
            Some("ctrl+pagedown")
        );
        assert_eq!(kitty_from_trigger("super+kp_1"), None);
    }

    #[test]
    fn kitty_shared_copy_rebind_and_file_are_stable() {
        let moves =
            kitty_with_shared_rebinds(&[movement("ctrl+shift+c", None, Some("copy_to_clipboard"))]);
        assert_eq!(moves[0].emit.as_deref(), Some("copy_or_noop"));
        let contents = kitty_keybinds_file(&moves);
        assert!(contents.contains("map ctrl+shift+c copy_or_noop"));
        let config = kitty_with_include("font_size 13");
        assert_eq!(config.matches(KITTY_INCLUDE_LINE).count(), 1);
        assert!(!kitty_without_include(&config).contains(KITTY_INCLUDE_LINE));
    }

    fn decision(choice: DecisionChoice) -> Decision {
        Decision {
            choice,
            key: None,
            action: None,
            guard: None,
            owner_terminal: false,
            command: None,
        }
    }

    #[test]
    fn claimant_bindings_remove_original_and_move_when_requested() {
        let mut choices = BTreeMap::new();
        let mut moved = decision(DecisionChoice::Terminal);
        moved.action = Some("extension.command".into());
        moved.key = Some("ctrl+alt+k".into());
        moved.guard = Some("editorTextFocus".into());
        choices.insert("claim:cmd+k:extension.command".into(), moved);
        let mut terminal_owned = decision(DecisionChoice::Terminal);
        terminal_owned.action = Some("terminal.action".into());
        terminal_owned.owner_terminal = true;
        choices.insert("claim:cmd+t:terminal.action".into(), terminal_owned);
        let bindings = claim_bindings(&Decisions {
            version: 1,
            terminal: "ghostty".into(),
            choices,
        });
        assert_eq!(
            bindings,
            vec![
                Binding {
                    key: "cmd+k".into(),
                    command: "-extension.command".into(),
                    when: None,
                },
                Binding {
                    key: "ctrl+alt+k".into(),
                    command: "extension.command".into(),
                    when: Some("editorTextFocus".into()),
                },
            ]
        );
    }

    #[test]
    fn imported_and_fallback_editor_moves_keep_commands() {
        let mut choices = BTreeMap::new();
        let mut imported = decision(DecisionChoice::Editor);
        imported.key = Some("cmd+alt+c".into());
        choices.insert("import:ctrl+c".into(), imported);
        let mut fallback = decision(DecisionChoice::Editor);
        fallback.key = Some("ctrl+alt+f".into());
        fallback.command = Some("workbench.action.find".into());
        choices.insert("cmd+f".into(), fallback);
        let decisions = Decisions {
            version: 1,
            terminal: "ghostty".into(),
            choices,
        };
        assert_eq!(
            override_bindings(&decisions, true)[0].command,
            "tode.confirmQuit"
        );
        assert_eq!(
            fallback_bindings(Some(&decisions))[0].command,
            "workbench.action.find"
        );
    }

    #[test]
    fn quit_and_hint_bindings_follow_platform_and_decisions() {
        assert_eq!(quit_chord(true), "ctrl+c");
        assert_eq!(quit_chord(false), "ctrl+q");
        assert!(hint_bindings(true).is_empty());
        assert_eq!(hint_bindings(false)[0].key, "ctrl+c");
        assert_eq!(
            quit_bindings(None, true)[0].when.as_deref(),
            Some("!terminalFocus && !editorHasSelection && (!inputFocus || editorTextFocus)")
        );

        let mut choices = BTreeMap::new();
        choices.insert("ctrl+c".into(), decision(DecisionChoice::Keep));
        let decisions = Decisions {
            version: 1,
            terminal: "ghostty".into(),
            choices,
        };
        assert!(quit_bindings(Some(&decisions), true).is_empty());
    }

    #[test]
    fn non_editor_or_namespaced_decisions_do_not_create_fallbacks() {
        let mut choices = BTreeMap::new();
        let mut claim = decision(DecisionChoice::Editor);
        claim.key = Some("ctrl+x".into());
        claim.command = Some("ignored".into());
        choices.insert("claim:cmd+x".into(), claim);
        choices.insert("cmd+y".into(), decision(DecisionChoice::Terminal));
        assert!(
            fallback_bindings(Some(&Decisions {
                version: 1,
                terminal: "kitty".into(),
                choices,
            }))
            .is_empty()
        );
    }
}
