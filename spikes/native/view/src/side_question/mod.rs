//! Side-question lifecycle and validated input. No UI, transport, or parent log.
//! The node owns context refresh; this state owns correlation and recovery.

mod question;
mod state;

pub use question::Question;
pub use state::{Operation, Pending, Problem, State, Target};
