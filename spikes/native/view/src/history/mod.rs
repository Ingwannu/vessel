//! Own-history import state with bounded retained metadata and explicit receipts.
mod bounds;
mod state;
#[cfg(test)]
mod tests;
mod types;
mod words;
pub use bounds::MAX_IMPORT_ENTRIES;
pub use state::{History, Loading, Selection};
pub use types::{Ask, Batch, Cursor, Entry, Listed, Outcome, Problem, Reply, Scope};
