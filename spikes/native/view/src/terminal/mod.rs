//! A terminal session's view logic: keys and sizes sent in order, when a
//! view may resize, what the frame says, the decoding of a program's
//! bytes into typed actions (`parse`), the grid they draw on (`grid`,
//! `model`), its colours as drawn (`paint`), and the closed set of queries
//! it answers and which viewer answers them (`reply`, `duty`), ADR 0073.

mod duty;
mod frame;
pub mod grid;
mod model;
pub mod paint;
pub mod parse;
mod reply;
mod sender;
mod size;
mod viewport;

pub use duty::{Focus, Origin, Others, QueryDuty, Typed, query_duty};
pub use frame::{Liveness, ResizeRule, TerminalLog};
pub use model::{Feed, TerminalModel};
pub use reply::{LONGEST_REPLY, REPLY_RATIO};
pub use sender::{MAX_PENDING, Outgoing, Taken, TerminalSender};
pub use size::{GridSize, ScrollbackLines, TerminalError};
pub use viewport::Viewport;
