//! A terminal session's view logic: keys and sizes sent in order, when a
//! view may resize, what the frame says.

mod frame;
mod sender;

pub use frame::{Liveness, ResizeRule, TerminalLog};
pub use sender::{MAX_PENDING, Outgoing, Taken, TerminalSender};
