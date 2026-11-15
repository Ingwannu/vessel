//! Explicit snapshot/change mode and contiguous catalog update identity.

use serde::{Deserialize, Serialize};

use super::{Label, LabelName};
use crate::{WorkspaceLabelGeneration, WorkspaceLabelSeq};

/// Resume identity is scoped to one catalog service generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CatalogCursor {
    /// New process/service generation requires a snapshot.
    pub generation: WorkspaceLabelGeneration,
    /// Last fully applied update.
    pub after: WorkspaceLabelSeq,
}

/// A committed mutation, with previous spelling only when renamed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CatalogDelta {
    /// Add or replace a host definition.
    Upsert {
        /// Committed definition.
        label: Label,
        /// Prior identity is removed before installing the new one.
        previous: Option<LabelName>,
    },
    /// Remove a committed definition.
    Remove {
        /// Deleted spelling.
        name: LabelName,
    },
}

/// One generation-scoped live notification, emitted only after commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CatalogUpdate {
    /// Catalog sequence and generation.
    pub cursor: CatalogCursor,
    /// Committed mutation.
    pub delta: CatalogDelta,
}

/// A snapshot replaces state; changes extend only the requested cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CatalogMode {
    /// Full replacement when there is no usable retained cursor.
    Snapshot {
        /// Every current definition.
        labels: Vec<Label>,
    },
    /// Retained contiguous mutations; preserve the original base identity.
    Changes {
        /// Cursor this batch extends.
        from: CatalogCursor,
        /// Every update through the answer's head.
        updates: Vec<CatalogUpdate>,
    },
}

/// The atomically observed catalog state or catch-up ending at `head`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CatalogList {
    /// Snapshot/change payload.
    pub data: CatalogMode,
    /// Final committed generation and sequence covered by the payload.
    pub head: CatalogCursor,
}
