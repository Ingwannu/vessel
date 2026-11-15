//! Attribution is explicit because native tool ids may repeat in later turns.
use super::ExternalTurnId;
use crate::EventSeq;
use serde::{Deserialize, Serialize};

/// Scope paired with a tool id; it grants no provider or node authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolCallScope {
    /// The provider supplied its actual turn identity, including on late updates.
    ProviderTurn {
        /// Identity in this provider session.
        turn: ExternalTurnId,
    },
    /// The actor observed the announcement inside this numbered session turn.
    SessionTurn {
        /// Sequence of the canonical turn-start record.
        started: EventSeq,
    },
    /// Attribution is unavailable or ambiguous; this record stands alone.
    Record {
        /// Sequence of this unassociated observation, never a guessed current turn.
        seq: EventSeq,
    },
}
