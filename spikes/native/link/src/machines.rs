//! Book-backed machine management using the CLI's atomic `BookFile` edits.
//!
//! [Decision Log]
//! - Purpose: one source for names/defaults shared with the CLI.
//! - Constraints: reread immediately before writes; never expose book contents
//!   in an error (a malformed entry may contain a credential).
//! - Alternatives: a second app store, or optimistic mutations in the screen.
//! - Choice: serialized commands, typed success/failure, then a fresh snapshot.
//! - Costs: file work runs on the blocking pool; selection changes only on success.

use vessel_app_view::machine_book::{Edit, Failure, Name};
use vessel_app_view::machines::Transport;
use vessel_proto::{NodeInfo, ProtocolVersion};

/// A saved machine with only non-secret connection metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    /// Validated book name.
    pub name: Name,
    /// Connection kind; no bearer token or credential path.
    pub transport: Transport,
    /// Whether new sessions use it by default.
    pub default: bool,
    /// What the established connection reported.
    pub node: Option<NodeInfo>,
    /// The negotiated protocol, if connected.
    pub protocol: Option<ProtocolVersion>,
}

/// What the Machines screen asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// Read the current book and connection facts.
    Read,
    /// Apply a confirmed edit.
    Edit(Edit),
    /// Retry connections now, then read the book.
    Retry,
}

/// The result of one operation. Original errors never cross into screen text.
#[derive(Debug)]
pub enum Answer {
    /// Snapshot of the saved list.
    Read(Result<Vec<Machine>, Failure>),
    /// A selection may follow this edit only when its result is successful.
    Edited {
        /// The edit that completed.
        edit: Edit,
        /// Durable write outcome.
        result: Result<(), Failure>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
mod edits;
#[cfg(not(target_arch = "wasm32"))]
mod native;
