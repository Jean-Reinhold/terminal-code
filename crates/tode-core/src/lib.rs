pub mod cli;
pub mod color;
pub mod ipc;
pub mod jsonc;
pub mod palette;
pub mod shortcuts;
pub mod target;
pub mod theme;

pub use cli::{HELP, installed_version};
pub use color::{
    Oklch, contrast, from_oklch, hex, is_dark, legible, luminance, mix, parse_hex, shade, to_oklch,
    with_alpha,
};
pub use ipc::{IpcError, OpenRequest, send_to_extension};
pub use jsonc::{parse_jsonc, read_key, set_key, set_keys};
pub use palette::{
    ParsedReplies, Rgb, TerminalPalette, build_query, parse_color, parse_replies, with_fallbacks,
};
pub use shortcuts::{
    FreedMove, GHOSTTY_INCLUDE_LINE, KITTY_INCLUDE_LINE, ParsedGhosttyTrigger, canonical_chord,
    emit_sequence, ghostty_from_trigger, ghostty_keybinds_file, ghostty_to_trigger,
    ghostty_with_include, ghostty_without_include, kitty_from_trigger, kitty_keybinds_file,
    kitty_to_trigger, kitty_with_include, kitty_with_shared_rebinds, kitty_without_include,
    normalize_chord, parse_ghostty_trigger,
};
pub use target::{OpenFile, Target, parse_goto, resolve_target};
pub use theme::{
    GeneratedTheme, SemanticColors, Surfaces, generate_theme, palette_fingerprint, semantic_colors,
    surfaces, theme_fingerprint,
};
