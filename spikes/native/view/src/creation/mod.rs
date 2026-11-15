//! Creation state shared by the draft, attachment handoff and node link.
//! Correlation is independent of node request ids: late work must never consume
//! another draft's text or attachments, including after a route change.

mod flow;
mod launch;
mod seed;
mod ticket;

pub use seed::{
    DraftDestination, DraftSeed, NewWorkspaceSeed, ProjectSeed, SeedPlace, SeedProblem,
    WorkspaceForm,
};

pub use launch::{Launch, LaunchPlan, LaunchProblem, PayloadPresence, ShellInput};

pub use flow::{Completion, Failure, Flow, Navigation, Stage, Transition};
pub use ticket::Ticket;
