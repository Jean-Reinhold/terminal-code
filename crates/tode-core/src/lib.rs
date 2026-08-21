pub mod ipc;
pub mod jsonc;
pub mod palette;
pub mod target;

pub use ipc::{IpcError, OpenRequest, send_to_extension};
pub use jsonc::{parse_jsonc, read_key, set_key, set_keys};
pub use palette::{
    ParsedReplies, Rgb, TerminalPalette, build_query, parse_color, parse_replies, with_fallbacks,
};
pub use target::{OpenFile, Target, parse_goto, resolve_target};
