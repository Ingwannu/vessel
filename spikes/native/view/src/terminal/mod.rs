//! A terminal session's view logic: keys and sizes sent in order, when a
//! view may resize, what the frame says, the decoding of a program's
//! bytes into typed actions (`parse`), the grid they draw on (`grid`,
//! `model`), and its colours as drawn (`paint`), ADR 0073.

mod frame;
pub mod grid;
mod model;
pub mod paint;
pub mod parse;
mod sender;
mod size;

pub use frame::{Liveness, ResizeRule, TerminalLog};
pub use model::{Feed, TerminalModel};
pub use sender::{MAX_PENDING, Outgoing, Taken, TerminalSender};
pub use size::{GridSize, ScrollbackLines, TerminalError};
