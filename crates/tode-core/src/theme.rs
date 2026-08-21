use std::collections::BTreeMap;
use std::f64::consts::PI;

use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    Rgb, TerminalPalette, contrast, hex, is_dark, legible, mix, shade, to_oklch, with_alpha,
};

pub const THEME_NAME: &str = "Terminal Code";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticColors {
    pub red: Rgb,
    pub green: Rgb,
    pub yellow: Rgb,
    pub blue: Rgb,
    pub magenta: Rgb,
    pub cyan: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surfaces {
    pub editor: Rgb,
    pub raised: Rgb,
    pub sunken: Rgb,
    pub overlay: Rgb,
    pub border: Rgb,
    pub hover: Rgb,
    pub active: Rgb,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeneratedTheme {
    pub name: String,
    #[serde(rename = "type")]
    pub theme_type: String,
    #[serde(rename = "semanticHighlighting")]
    pub semantic_highlighting: bool,
    pub colors: BTreeMap<String, String>,
    #[serde(rename = "tokenColors")]
    pub token_colors: Vec<Value>,
}

pub fn semantic_colors(palette: &TerminalPalette) -> SemanticColors {
    let bright = &palette.ansi[9..15];
    let dim = &palette.ansi[1..7];
    let pool = if bright.iter().any(|color| to_oklch(*color).c > 0.02) {
        bright
    } else {
        dim
    };
    let background = palette.background;
    SemanticColors {
        red: nearest_hue(pool, 29.0, background),
        green: nearest_hue(pool, 142.0, background),
        yellow: nearest_hue(pool, 90.0, background),
        blue: nearest_hue(pool, 264.0, background),
        magenta: nearest_hue(pool, 328.0, background),
        cyan: nearest_hue(pool, 195.0, background),
    }
}

pub fn surfaces(background: Rgb, foreground: Rgb) -> Surfaces {
    Surfaces {
        editor: background,
        raised: shade(background, 6.0),
        sunken: shade(background, -10.0),
        overlay: shade(background, 20.0),
        border: mix(background, foreground, 0.14),
        hover: mix(background, foreground, 0.08),
        active: mix(background, foreground, 0.16),
    }
}

pub fn palette_fingerprint(palette: &TerminalPalette) -> String {
    let mut parts = String::new();
    parts.push_str(&hex(palette.background));
    parts.push_str(&hex(palette.foreground));
    for color in palette.ansi {
        parts.push_str(&hex(color));
    }
    short_hash(parts.as_bytes())
}

pub fn theme_fingerprint<T: Serialize>(theme: &T) -> Result<String, serde_json::Error> {
    serde_json::to_vec(theme).map(|bytes| short_hash(&bytes))
}

pub fn generate_theme(palette: &TerminalPalette) -> GeneratedTheme {
    let bg = palette.background;
    let fg = palette.foreground;
    let dark = is_dark(bg);
    let surface = surfaces(bg, fg);
    let accent = semantic_colors(palette);
    let on = |color, target| hex(legible(color, surface.editor, target));
    let muted = hex(mix(fg, bg, 0.4));
    let faint = hex(mix(fg, bg, 0.62));
    let primary = legible(accent.blue, surface.editor, 3.0);
    let mut colors = BTreeMap::new();
    macro_rules! color {
        ($name:expr, $value:expr) => {
            colors.insert($name.to_owned(), $value);
        };
    }

    color!("focusBorder", with_alpha(primary, 0.6));
    color!("foreground", hex(fg));
    color!("descriptionForeground", muted.clone());
    color!("errorForeground", on(accent.red, 4.5));
    color!("widget.border", hex(surface.border));
    color!("widget.shadow", with_alpha(shade(bg, -24.0), 0.4));
    color!("selection.background", with_alpha(primary, 0.35));
    color!("icon.foreground", muted.clone());
    color!("sash.hoverBorder", with_alpha(primary, 0.7));
    color!("editor.background", hex(surface.editor));
    color!("editor.foreground", hex(fg));
    color!("editorLineNumber.foreground", hex(mix(fg, bg, 0.68)));
    color!("editorLineNumber.activeForeground", hex(fg));
    color!("editorCursor.foreground", hex(primary));
    color!("editor.selectionBackground", with_alpha(primary, 0.3));
    color!(
        "editor.inactiveSelectionBackground",
        with_alpha(primary, 0.16)
    );
    color!(
        "editor.selectionHighlightBackground",
        with_alpha(primary, 0.16)
    );
    color!("editor.wordHighlightBackground", with_alpha(primary, 0.14));
    color!(
        "editor.wordHighlightStrongBackground",
        with_alpha(accent.green, 0.16)
    );
    color!("editor.findMatchBackground", with_alpha(accent.yellow, 0.4));
    color!(
        "editor.findMatchHighlightBackground",
        with_alpha(accent.yellow, 0.22)
    );
    color!("editor.lineHighlightBackground", hex(surface.raised));
    color!("editor.rangeHighlightBackground", with_alpha(primary, 0.1));
    color!("editorWhitespace.foreground", with_alpha(fg, 0.15));
    color!("editorIndentGuide.background1", with_alpha(fg, 0.1));
    color!("editorIndentGuide.activeBackground1", with_alpha(fg, 0.26));
    color!("editorRuler.foreground", hex(surface.border));
    color!("editorBracketMatch.background", with_alpha(primary, 0.2));
    color!("editorBracketMatch.border", with_alpha(primary, 0.5));
    color!("editorError.foreground", on(accent.red, 4.5));
    color!("editorWarning.foreground", on(accent.yellow, 4.5));
    color!("editorInfo.foreground", on(accent.blue, 4.5));
    color!("editorGutter.addedBackground", on(accent.green, 3.0));
    color!("editorGutter.modifiedBackground", on(accent.blue, 3.0));
    color!("editorGutter.deletedBackground", on(accent.red, 3.0));
    color!("editorOverviewRuler.border", "#00000000".into());
    color!("editorLink.activeForeground", hex(primary));
    color!("editorWidget.background", hex(surface.overlay));
    color!("editorWidget.border", hex(surface.border));
    color!("editorSuggestWidget.background", hex(surface.overlay));
    color!("editorSuggestWidget.border", hex(surface.border));
    color!(
        "editorSuggestWidget.selectedBackground",
        hex(surface.active)
    );
    color!("editorSuggestWidget.highlightForeground", hex(primary));
    color!("editorHoverWidget.background", hex(surface.overlay));
    color!("editorHoverWidget.border", hex(surface.border));
    color!("quickInput.background", hex(surface.overlay));
    color!("quickInput.foreground", hex(fg));
    color!("quickInputList.focusBackground", hex(surface.active));
    color!("quickInputTitle.background", hex(surface.overlay));
    color!("pickerGroup.foreground", muted.clone());
    color!("pickerGroup.border", hex(surface.border));
    color!("sideBar.background", hex(surface.sunken));
    color!("sideBar.foreground", hex(mix(fg, bg, 0.12)));
    color!("sideBar.border", hex(surface.border));
    color!("sideBarTitle.foreground", muted.clone());
    color!("sideBarSectionHeader.background", hex(surface.sunken));
    color!("sideBarSectionHeader.foreground", muted.clone());
    color!("sideBarSectionHeader.border", hex(surface.border));
    color!("activityBar.background", hex(surface.sunken));
    color!("activityBar.foreground", hex(fg));
    color!("activityBar.inactiveForeground", faint.clone());
    color!("activityBar.border", hex(surface.border));
    color!("activityBarBadge.background", hex(primary));
    color!(
        "activityBarBadge.foreground",
        hex(if is_dark(primary) {
            [255, 255, 255]
        } else {
            [0, 0, 0]
        })
    );
    color!("statusBar.background", hex(surface.sunken));
    color!("statusBar.foreground", muted.clone());
    color!("statusBar.border", hex(surface.border));
    color!("statusBar.noFolderBackground", hex(surface.sunken));
    color!("statusBar.debuggingBackground", hex(accent.yellow));
    color!(
        "statusBar.debuggingForeground",
        hex(if is_dark(accent.yellow) {
            [255, 255, 255]
        } else {
            [0, 0, 0]
        })
    );
    color!("statusBarItem.remoteBackground", hex(surface.sunken));
    color!("statusBarItem.remoteForeground", muted.clone());
    color!("statusBarItem.hoverBackground", hex(surface.hover));
    color!("titleBar.activeBackground", hex(surface.sunken));
    color!("titleBar.activeForeground", muted.clone());
    color!("titleBar.inactiveBackground", hex(surface.sunken));
    color!("titleBar.inactiveForeground", faint.clone());
    color!("titleBar.border", hex(surface.border));
    color!("panel.background", hex(surface.editor));
    color!("panel.border", hex(surface.border));
    color!("panelTitle.activeForeground", hex(fg));
    color!("panelTitle.inactiveForeground", faint.clone());
    color!("panelTitle.activeBorder", hex(primary));
    color!("editorGroupHeader.tabsBackground", hex(surface.sunken));
    color!("editorGroupHeader.tabsBorder", hex(surface.border));
    color!("editorGroupHeader.noTabsBackground", hex(surface.sunken));
    color!("editorGroup.border", hex(surface.border));
    color!("tab.activeBackground", hex(surface.editor));
    color!("tab.activeForeground", hex(fg));
    color!("tab.inactiveBackground", hex(surface.sunken));
    color!("tab.inactiveForeground", faint.clone());
    color!("tab.border", hex(surface.border));
    color!("tab.activeBorderTop", hex(primary));
    color!("tab.hoverBackground", hex(surface.hover));
    color!("tab.unfocusedActiveBackground", hex(surface.sunken));
    color!("list.activeSelectionBackground", hex(surface.active));
    color!("list.activeSelectionForeground", hex(fg));
    color!("list.inactiveSelectionBackground", hex(surface.hover));
    color!("list.hoverBackground", hex(surface.hover));
    color!("list.focusBackground", hex(surface.active));
    color!("list.highlightForeground", hex(primary));
    color!("list.errorForeground", on(accent.red, 4.5));
    color!("list.warningForeground", on(accent.yellow, 4.5));
    color!("tree.indentGuidesStroke", with_alpha(fg, 0.16));
    color!("input.background", hex(surface.raised));
    color!("input.foreground", hex(fg));
    color!("input.border", hex(surface.border));
    color!("input.placeholderForeground", faint.clone());
    color!("inputOption.activeBorder", hex(primary));
    color!(
        "inputValidation.errorBackground",
        hex(mix(bg, accent.red, 0.3))
    );
    color!("inputValidation.errorBorder", on(accent.red, 3.0));
    color!("dropdown.background", hex(surface.overlay));
    color!("dropdown.foreground", hex(fg));
    color!("dropdown.border", hex(surface.border));
    color!("button.background", hex(primary));
    color!(
        "button.foreground",
        hex(if is_dark(primary) {
            [255, 255, 255]
        } else {
            [0, 0, 0]
        })
    );
    color!("button.hoverBackground", hex(shade(primary, 12.0)));
    color!("button.secondaryBackground", hex(surface.active));
    color!("button.secondaryForeground", hex(fg));
    color!("badge.background", hex(surface.active));
    color!("badge.foreground", hex(fg));
    color!("progressBar.background", hex(primary));
    color!("scrollbar.shadow", "#00000000".into());
    color!("scrollbarSlider.background", with_alpha(fg, 0.14));
    color!("scrollbarSlider.hoverBackground", with_alpha(fg, 0.22));
    color!("scrollbarSlider.activeBackground", with_alpha(fg, 0.3));
    color!("minimap.background", hex(surface.editor));
    color!("menu.background", hex(surface.overlay));
    color!("menu.foreground", hex(fg));
    color!("menu.border", hex(surface.border));
    color!("menu.selectionBackground", hex(surface.active));
    color!("menubar.selectionBackground", hex(surface.hover));
    color!("notificationCenterHeader.background", hex(surface.overlay));
    color!("notifications.background", hex(surface.overlay));
    color!("notifications.border", hex(surface.border));
    color!(
        "gitDecoration.modifiedResourceForeground",
        on(accent.yellow, 3.0)
    );
    color!(
        "gitDecoration.deletedResourceForeground",
        on(accent.red, 3.0)
    );
    color!(
        "gitDecoration.untrackedResourceForeground",
        on(accent.green, 3.0)
    );
    color!("gitDecoration.ignoredResourceForeground", faint.clone());
    color!(
        "gitDecoration.conflictingResourceForeground",
        on(accent.magenta, 3.0)
    );
    color!("peekView.border", hex(primary));
    color!("peekViewEditor.background", hex(surface.raised));
    color!("peekViewResult.background", hex(surface.sunken));
    color!("peekViewTitle.background", hex(surface.sunken));
    color!("breadcrumb.foreground", faint.clone());
    color!("breadcrumb.focusForeground", hex(fg));
    color!("breadcrumb.background", hex(surface.editor));
    color!("terminal.background", hex(surface.editor));
    color!("terminal.foreground", hex(fg));
    color!("terminalCursor.foreground", hex(primary));
    color!("terminal.selectionBackground", with_alpha(primary, 0.3));
    color!("terminal.border", hex(surface.border));

    const ANSI_NAMES: [&str; 16] = [
        "Black",
        "Red",
        "Green",
        "Yellow",
        "Blue",
        "Magenta",
        "Cyan",
        "White",
        "BrightBlack",
        "BrightRed",
        "BrightGreen",
        "BrightYellow",
        "BrightBlue",
        "BrightMagenta",
        "BrightCyan",
        "BrightWhite",
    ];
    for (index, color_value) in palette.ansi.into_iter().enumerate() {
        color!(
            &format!("terminal.ansi{}", ANSI_NAMES[index]),
            hex(color_value)
        );
    }

    let token = |scopes: &[&str], color: Rgb, style: Option<&str>| {
        let mut settings = serde_json::Map::new();
        settings.insert("foreground".into(), json!(on(color, 4.5)));
        if let Some(style) = style {
            settings.insert("fontStyle".into(), json!(style));
        }
        json!({"scope": scopes, "settings": settings})
    };
    let token_colors = vec![
        json!({
            "scope": ["comment", "punctuation.definition.comment"],
            "settings": {
                "foreground": hex(legible(mix(fg, bg, 0.5), surface.editor, 3.0)),
                "fontStyle": "italic"
            }
        }),
        token(
            &["keyword", "storage", "storage.type", "keyword.control"],
            accent.red,
            None,
        ),
        token(
            &["string", "string.quoted", "punctuation.definition.string"],
            accent.green,
            None,
        ),
        token(
            &[
                "constant.numeric",
                "constant.language",
                "constant.character",
            ],
            accent.magenta,
            None,
        ),
        token(
            &[
                "entity.name.function",
                "support.function",
                "meta.function-call",
            ],
            accent.blue,
            None,
        ),
        token(
            &[
                "entity.name.type",
                "entity.name.class",
                "support.class",
                "support.type",
            ],
            accent.yellow,
            None,
        ),
        token(
            &[
                "variable",
                "meta.definition.variable.name",
                "variable.other.readwrite",
            ],
            fg,
            None,
        ),
        token(&["variable.parameter"], mix(fg, accent.cyan, 0.5), None),
        token(&["entity.name.tag"], accent.red, None),
        token(&["entity.other.attribute-name"], accent.yellow, None),
        token(
            &["support.type.property-name", "meta.object-literal.key"],
            accent.cyan,
            None,
        ),
        token(&["punctuation", "meta.brace"], mix(fg, bg, 0.3), None),
        token(&["invalid"], accent.red, None),
        json!({"scope": ["markup.heading"], "settings": {"foreground": on(accent.blue, 4.5), "fontStyle": "bold"}}),
        json!({"scope": ["markup.italic"], "settings": {"fontStyle": "italic"}}),
        json!({"scope": ["markup.bold"], "settings": {"fontStyle": "bold"}}),
    ];

    GeneratedTheme {
        name: THEME_NAME.into(),
        theme_type: if dark { "dark" } else { "light" }.into(),
        semantic_highlighting: true,
        colors,
        token_colors,
    }
}

fn nearest_hue(palette: &[Rgb], degrees: f64, background: Rgb) -> Rgb {
    let target = degrees * PI / 180.0;
    let mut best = palette[0];
    let mut best_score = f64::INFINITY;
    for color in palette {
        let converted = to_oklch(*color);
        if converted.c < 0.02 {
            continue;
        }
        let delta = (converted.h - target)
            .sin()
            .atan2((converted.h - target).cos())
            .abs();
        let score = delta - contrast(*color, background).min(8.0) * 0.02;
        if score < best_score {
            best_score = score;
            best = *color;
        }
    }
    best
}

fn short_hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))[..16].to_owned()
}

#[cfg(test)]
mod tests {
    use crate::{parse_hex, with_fallbacks};

    use super::*;

    fn black() -> TerminalPalette {
        with_fallbacks(None)
    }

    #[test]
    fn black_and_light_terminals_select_expected_type_and_editor() {
        let mut black = black();
        black.background = [0, 0, 0];
        black.foreground = [255, 255, 255];
        let dark = generate_theme(&black);
        assert_eq!(dark.theme_type, "dark");
        assert_eq!(dark.colors["editor.background"], "#000000");

        let mut light = black;
        light.background = [255, 255, 255];
        light.foreground = [30, 30, 30];
        let light = generate_theme(&light);
        assert_eq!(light.theme_type, "light");
        assert_eq!(light.colors["editor.background"], "#ffffff");
    }

    #[test]
    fn extreme_backgrounds_keep_surfaces_separate() {
        for background in [[0, 0, 0], [255, 255, 255]] {
            let surface = surfaces(
                background,
                if background[0] == 0 {
                    [255, 255, 255]
                } else {
                    [0, 0, 0]
                },
            );
            assert_ne!(surface.editor, surface.raised);
            assert_ne!(surface.editor, surface.sunken);
            assert_ne!(surface.raised, surface.overlay);
        }
    }

    #[test]
    fn ansi_palette_is_carried_verbatim() {
        let palette = black();
        let theme = generate_theme(&palette);
        assert_eq!(theme.colors["terminal.ansiRed"], hex(palette.ansi[1]));
        assert_eq!(
            theme.colors["terminal.ansiBrightBlue"],
            hex(palette.ansi[12])
        );
    }

    #[test]
    fn text_colours_reach_wcag_aa_on_editor() {
        for palette in [black()] {
            let theme = generate_theme(&palette);
            let background = parse_hex(&theme.colors["editor.background"]).unwrap();
            for key in [
                "errorForeground",
                "editorError.foreground",
                "editorWarning.foreground",
                "editorInfo.foreground",
                "list.errorForeground",
                "list.warningForeground",
            ] {
                let foreground = parse_hex(&theme.colors[key][..7]).unwrap();
                assert!(contrast(foreground, background) >= 4.5, "{key}");
            }
        }
    }

    #[test]
    fn fingerprints_follow_palette_and_theme() {
        let first = black();
        let mut second = first.clone();
        second.background = [1, 0, 0];
        assert_eq!(palette_fingerprint(&first), palette_fingerprint(&first));
        assert_ne!(palette_fingerprint(&first), palette_fingerprint(&second));
        assert_ne!(
            theme_fingerprint(&generate_theme(&first)).unwrap(),
            theme_fingerprint(&generate_theme(&second)).unwrap()
        );
    }

    #[test]
    fn generated_theme_contains_complete_workbench_and_tokens() {
        let theme = generate_theme(&black());
        assert!(theme.colors.len() > 140);
        assert_eq!(theme.token_colors.len(), 16);
        assert!(theme.semantic_highlighting);
    }
}
