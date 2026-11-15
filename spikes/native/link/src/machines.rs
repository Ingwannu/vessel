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
    /// Every route to it, in the order they are tried; endpoints only.
    pub routes: Vec<vessel_app_view::connections::Route>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl Ask {
    /// Refuse in the same response shape as the request, including on the browser.
    #[must_use]
    pub fn refused(self, why: Failure) -> Answer {
        match self {
            Self::Read | Self::Retry => Answer::Read(Err(why)),
            Self::Edit(edit) => Answer::Edited {
                edit,
                result: Err(why),
            },
        }
    }
}

/// The app's machine-list path, resolved once at startup. A test supplies an
/// isolated path through the same type; it never changes process-wide env.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub(crate) enum BookPath {
    File(std::path::PathBuf),
    NoConfiguration,
}
#[cfg(not(target_arch = "wasm32"))]
impl BookPath {
    pub(crate) fn discover() -> Self {
        match crate::nodes::config_dir() {
            Ok(dir) => Self::File(vessel_client::book::NodeBook::path_in(&dir)),
            Err(_no_configuration) => Self::NoConfiguration,
        }
    }
    pub(super) fn path(&self) -> Result<&std::path::Path, Failure> {
        match self {
            Self::File(path) => Ok(path),
            Self::NoConfiguration => Err(Failure::NoConfiguration),
        }
    }
}
#[cfg(test)]
mod tests;
