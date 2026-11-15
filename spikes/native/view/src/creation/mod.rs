//! Creation state shared by the draft, attachment handoff and node link.
//! Correlation is independent of node request ids: late work must never consume
//! another draft's text or attachments, including after a route change.

mod flow;
mod ticket;

pub use flow::{Completion, Failure, Flow, Navigation, Stage, Transition};
pub use ticket::Ticket;
