//! Own-history import state with bounded retained metadata and explicit receipts.
mod bounds;
mod sheet;
mod types;
mod words;
pub use bounds::MAX_IMPORT_ENTRIES;
pub use sheet::{Answer, Empty, FeedState, Filter, Opening, PAGE_LIMITS, Row, Sheet};
pub use types::{Ask, Batch, Cursor, Entry, Listed, Outcome, Problem, Reply, Scope};
