//! Retained-state reservations bound durable replay protection without eviction.
//! The store writer owns atomic admission; this module owns units and budgets.

use serde::{Deserialize, Serialize};

/// A long-lived session refuses fresh identities after this retained record count.
pub const MAX_SESSION_ADMISSIONS: u64 = 8192;
/// Global retained records remain finite across many retained sessions.
pub const MAX_NODE_ADMISSIONS: u64 = 131_072;
/// Session reservations include future accepted results and question resolutions.
pub const MAX_SESSION_ADMISSION_BYTES: u64 = 16 * 1024 * 1024;
/// Logical retained-state ceiling, separate from `SQLite` page/WAL overhead.
pub const MAX_NODE_ADMISSION_BYTES: u64 = 256 * 1024 * 1024;
/// Receipt keys, creator, fingerprint and the largest outcome fit this envelope.
pub const MAX_RECEIPT_STATE_BYTES: u64 = 4 * 1024;
/// Question state includes typed options, previews, answer and resolution data.
pub const MAX_QUESTION_STATE_BYTES: u64 = 512 * 1024;
/// Each reservation charges fixed logical metadata in addition to its envelope.
pub const ADMISSION_ROW_CHARGE_BYTES: u64 = 512;

/// The capacity owner is explicit so clients do not retry a node-wide refusal per session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AdmissionScope {
    /// The owning session's retained state is full.
    Session,
    /// The node's retained state across all sessions is full.
    Node,
}

/// Capacity refuses before work; bytes and record count are independent budgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AdmissionResource {
    /// Too many retained replay identities.
    Records,
    /// Too much reserved logical encoded state.
    Bytes,
}

/// A typed refusal never claims that a prompt or answer was accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[error("retained admission capacity exhausted ({scope:?}, {resource:?})")]
pub struct AdmissionCapacity {
    /// The owner whose finite budget would be exceeded.
    pub scope: AdmissionScope,
    /// The budget which refused the reservation.
    pub resource: AdmissionResource,
}

/// The kind fixes the complete reservation before an external effect is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AdmissionKind {
    /// An identified prompt's Pending and Accepted outcomes share one reservation.
    Receipt,
    /// A durable question and every answer/admission/tombstone state share one reservation.
    Question,
}

impl AdmissionKind {
    /// The entire reserved charge, including fixed metadata, never shrinks on settlement.
    #[must_use]
    pub const fn reserved_bytes(self) -> u64 {
        let envelope = match self {
            Self::Receipt => MAX_RECEIPT_STATE_BYTES,
            Self::Question => MAX_QUESTION_STATE_BYTES,
        };
        // Constant envelopes cannot overflow, and saturation preserves refusal if expanded.
        envelope.saturating_add(ADMISSION_ROW_CHARGE_BYTES)
    }

    /// Check serialized state against its already reserved envelope before effects.
    #[must_use]
    pub const fn fits(self, encoded_bytes: u64) -> bool {
        let maximum = match self {
            Self::Receipt => MAX_RECEIPT_STATE_BYTES,
            Self::Question => MAX_QUESTION_STATE_BYTES,
        };
        encoded_bytes <= maximum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_reservation_covers_its_full_encoded_envelope_and_metadata() {
        for (kind, maximum) in [
            (AdmissionKind::Receipt, MAX_RECEIPT_STATE_BYTES),
            (AdmissionKind::Question, MAX_QUESTION_STATE_BYTES),
        ] {
            assert!(kind.fits(maximum));
            assert!(!kind.fits(maximum.checked_add(1).unwrap()));
            assert_eq!(
                kind.reserved_bytes(),
                maximum.checked_add(ADMISSION_ROW_CHARGE_BYTES).unwrap()
            );
        }
    }
}
