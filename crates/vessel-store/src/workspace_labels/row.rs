//! Which stored label row failed to decode, never what it holds.
//!
//! [Decision Log]
//! - Purpose: one corrupt label row refuses node startup, as Paseo's daemon
//!   stops on a catalog it cannot parse (`bootstrap.ts` 1070 awaits
//!   `initialize()` uncaught; `catalog-store.ts` 171-178 parses the whole
//!   array). The operator must still find the row (coordinator ruling on the
//!   A6 review's note b, 2026-11-16).
//! - Constraint: both label tables are `WITHOUT ROWID` and keyed by the
//!   normalized label name, so the key itself is stored content.
//! - Choice: the row's position in the order its reader uses (the catalog by
//!   key, a workspace's assignments by position), counted from 1, and for an
//!   assignment its workspace id. Neither repeats a name or a color.
//! - Rejected: the key (content); a hash of it (guessable for short names);
//!   the decoder's own error (serde echoes the value it refused).

use std::{fmt, num::NonZeroUsize};

use vessel_proto::WorkspaceId;

/// A stored label row, named without its contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRow {
    /// The n-th row of `workspace_label_catalog` in key order, from 1.
    Catalog(NonZeroUsize),
    /// The n-th row of one workspace's `workspace_label_assignments` in
    /// assignment order, from 1.
    Assignment {
        /// The workspace the row belongs to.
        workspace: WorkspaceId,
        /// Its place in that workspace's order, from 1.
        ordinal: NonZeroUsize,
    },
}

impl LabelRow {
    /// The catalog row read at zero-based `index` in key order.
    pub(crate) const fn catalog(index: usize) -> Self {
        Self::Catalog(NonZeroUsize::MIN.saturating_add(index))
    }

    /// `workspace`'s assignment read at zero-based `index` in its order.
    pub(crate) const fn assignment(workspace: WorkspaceId, index: usize) -> Self {
        Self::Assignment {
            workspace,
            ordinal: NonZeroUsize::MIN.saturating_add(index),
        }
    }
}

impl fmt::Display for LabelRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(ordinal) => {
                write!(f, "workspace_label_catalog row {ordinal} in key order")
            }
            Self::Assignment { workspace, ordinal } => write!(
                f,
                "workspace_label_assignments row {ordinal} of workspace {workspace} in assignment order"
            ),
        }
    }
}
