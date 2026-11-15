//! Diagnostic source admission, separate from wire scope/version admission.
use serde::{Deserialize, Serialize};
/// Node-wide live collections, including cancelled callers with retained I/O.
pub const MAX_DIAGNOSTIC_COLLECTIONS: usize = 2;
/// Safe typed explanation of resource admission refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DiagnosticRefusal {
    /// All named collection slots are retained.
    Busy,
    /// The node is stopping and admits no source work.
    Stopping,
}
