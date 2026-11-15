//! Importing conversations without teaching screens an agent's file format.
//! Listing and importing retain the same account and directory (ADR 0053).

mod bounds;
mod fork_summary;
mod state;
#[cfg(test)]
mod tests;
mod types;
mod words;

pub use bounds::MAX_IMPORT_ENTRIES;
pub use state::{History, Loading, Selection};
pub use types::{Ask, Batch, Cursor, Entry, Listed, Outcome, Problem, Reply, Scope};

pub use fork_summary::{
    ForkBoundary, ForkSummary, MAX_FORK_UNITS, SummaryEntry, SummaryHistory, SummaryProblem,
    SummarySource, fork_summary,
};
