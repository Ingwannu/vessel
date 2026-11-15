//! Durable anchors identify a delivered turn without changing provider events.
use super::{CheckpointId, PathCount, RestoreId};
use crate::EventSeq;
use serde::{Deserialize, Serialize};

/// Metadata for files-only checkpoints; never conversation content or file bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckpointEvent {
    /// The actor accepted a real new prompt before asking for its before-image.
    /// This event's own durable sequence is `Boundary::BeforeTurn.prompt`.
    PromptAccepted,
    /// Link that anchor to the actual provider turn once its start is published.
    /// Incidental provider output cannot shift the checkpoint onto another turn.
    TurnAssociated {
        /// Durable accepted-prompt anchor.
        prompt: EventSeq,
        /// Actual `AgentEvent::TurnStarted` log position.
        turn: EventSeq,
    },
    /// Emitted only after verified filesystem restoration and durable outcome.
    FilesRestored {
        /// Durable operation whose reconnect status remains authoritative.
        restore: RestoreId,
        /// The selected file checkpoint; conversation history is unchanged.
        checkpoint: CheckpointId,
        /// Number of paths verified at the effective desired image.
        paths: PathCount,
    },
}
impl CheckpointEvent {
    /// A harmless older-peer representation, with no paths, file bytes or contents.
    #[must_use]
    pub const fn log_line(&self) -> &'static str {
        match self {
            Self::PromptAccepted => "vessel: a file checkpoint boundary was accepted",
            Self::TurnAssociated { .. } => {
                "vessel: a file checkpoint boundary belongs to this turn"
            }
            Self::FilesRestored { .. } => {
                "vessel: checkpoint files were restored; the conversation is unchanged"
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkpoint_events_are_pending_metadata_and_downgrade_for_every_current_peer() {
        let events = [
            CheckpointEvent::PromptAccepted,
            CheckpointEvent::TurnAssociated {
                prompt: EventSeq::new(2),
                turn: EventSeq::new(8),
            },
            CheckpointEvent::FilesRestored {
                restore: RestoreId::random(),
                checkpoint: CheckpointId::random(),
                paths: PathCount::new(1).unwrap(),
            },
        ];
        for event in events {
            let event = crate::SessionEvent::Checkpoint { event };
            assert_eq!(event.required_protocol(), None);
            let encoded = serde_json::to_string(&event).unwrap();
            assert_eq!(
                serde_json::from_str::<crate::SessionEvent>(&encoded).unwrap(),
                event
            );
            for protocol in [crate::ProtocolVersion::new(2), crate::PROTOCOL_VERSION] {
                assert!(matches!(
                    event.for_protocol(protocol),
                    Some(crate::SessionEvent::Agent {
                        event: crate::AgentEvent::HarnessLog { .. }
                    })
                ));
            }
        }
    }
}
