//! Private producer correlation, separate from public client message identities.
//! A decoded key is evidence only: current execution, parentage and grant must be
//! rechecked together at admission. Serialized intent cannot supply that fence.

use serde::{Deserialize, Serialize};

use crate::{EventSeq, FinishIntentId, NodeId, PermissionId, QuestionId};

/// A bounded internal replay key; neither public submit nor observation accepts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct NoticeKey {
    /// Actual local retained-state owner, never an inferred remote principal.
    pub owner: NodeId,
    /// Immutable node-minted intent, distinct from a user client message id.
    pub intent: FinishIntentId,
    /// Exact source fact retained before delivery is attempted.
    pub checkpoint: NoticeCheckpoint,
}

/// Request identities retain their own closed namespace and response authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
pub enum NoticeRequest {
    /// A real pending permission, not a guessed tool identity.
    Permission(PermissionId),
    /// A real pending typed question.
    Question(QuestionId),
}

/// Source sequences are actual retained facts, never synthesized from snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NoticeCheckpoint {
    /// One exact permission/question checkpoint in the observed execution.
    NeedsYou {
        /// Original typed pending request.
        request: NoticeRequest,
        /// Original canonical request record.
        source: EventSeq,
    },
    /// The observed execution's actual terminal record.
    Terminal {
        /// Original canonical completion record.
        source: EventSeq,
    },
    /// Irrevocably ends observation; a later terminal cannot reuse another slot.
    ObservationUnavailable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::admission::MAX_NOTICE_KEY_BYTES;

    #[test]
    fn every_checkpoint_roundtrips_within_the_reserved_key_bound() {
        let permission = PermissionId::random();
        let question = QuestionId::random();
        for checkpoint in [
            NoticeCheckpoint::NeedsYou {
                request: NoticeRequest::Permission(permission),
                source: EventSeq::new(u64::MAX),
            },
            NoticeCheckpoint::NeedsYou {
                request: NoticeRequest::Question(question),
                source: EventSeq::new(u64::MAX),
            },
            NoticeCheckpoint::Terminal {
                source: EventSeq::new(u64::MAX),
            },
            NoticeCheckpoint::ObservationUnavailable,
        ] {
            let key = NoticeKey {
                owner: NodeId::random(),
                intent: FinishIntentId::random(),
                checkpoint,
            };
            let encoded = serde_json::to_vec(&key).unwrap();
            assert!(u64::try_from(encoded.len()).unwrap() <= MAX_NOTICE_KEY_BYTES);
            assert_eq!(serde_json::from_slice::<NoticeKey>(&encoded).unwrap(), key);
        }
    }

    #[test]
    fn request_namespace_and_intent_remain_part_of_checkpoint_identity() {
        let spelling = "00000000-0000-4000-8000-000000000001";
        assert_ne!(
            NoticeRequest::Permission(spelling.parse().unwrap()),
            NoticeRequest::Question(spelling.parse().unwrap())
        );
        let key = NoticeKey {
            owner: NodeId::random(),
            intent: FinishIntentId::random(),
            checkpoint: NoticeCheckpoint::ObservationUnavailable,
        };
        assert_ne!(
            key,
            NoticeKey {
                intent: FinishIntentId::random(),
                ..key
            }
        );
        assert!(serde_json::from_str::<NoticeKey>(r#"{"owner":"invalid","intent":"invalid","checkpoint":{"kind":"observation_unavailable"}}"#).is_err());
    }
}
