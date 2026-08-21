pub mod ipc;
pub mod target;

pub use ipc::{IpcError, OpenRequest, send_to_extension};
pub use target::{OpenFile, Target, parse_goto, resolve_target};
