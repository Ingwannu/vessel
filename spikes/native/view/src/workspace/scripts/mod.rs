//! Project scripts use the node's supervisor; this module owns only the
//! visible snapshot, correlated intentions and safe browser destinations.
//! Approval remains the project's exact-digest policy (ADR 0031).
mod preview;
mod retirement;
mod rows;
pub mod services;
mod state;
pub mod words;
pub use preview::{PreviewHost, PreviewProblem, PreviewUrl};
pub use retirement::RetiredWrite;
pub use rows::{Entry, RunState, latest_run, listeners, retained_runs, run_state};
pub use state::{Action, ActionTicket, Issue, Model, Preview, Snapshot, Surface, Target};

/// Refresh only a visible project; background projects need no polling.
pub const REFRESH_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

#[cfg(test)]
mod tests;
