//! Side conversations (protocol 19, ADR 0045). Their parent link never
//! means resuming the parent's native conversation or writing into it.

use serde::{Deserialize, Serialize};

use crate::SessionId;
use crate::automation::PromptText;

/// Open or find the newest unarchived side conversation. Needs read and
/// operate; only people, including delegated people, may call it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OpenSideQuestion {
    /// The original structured session, never another side.
    pub session: SessionId,
}

/// Ask in a side conversation with a fresh bounded parent snapshot.
/// General prompt and queue ingress refuses sides to preserve this rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptSideQuestion {
    /// The side's id, not its parent's.
    pub session: SessionId,
    /// A nonblank question of at most 16 KiB.
    pub question: PromptText,
}

/// Replace the expected side under the same creation gate. Deletion and
/// creation are serialized, not a transaction with rollback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResetSideQuestion {
    /// The original session.
    pub session: SessionId,
    /// The side the person saw. A stale id must not delete a newer side.
    pub side: SessionId,
}

/// Maximum records examined for the partial recent transcript.
pub const TRANSCRIPT_RECORDS: u32 = 128;
/// Maximum steps retained independently of the transcript's tail.
pub const PLAN_STEPS: usize = 20;
/// Maximum characters retained in a plan step or a transcript message.
pub const TEXT_CHARS: usize = 1_200;
/// Maximum recent conversation messages in a snapshot.
pub const TRANSCRIPT_MESSAGES: usize = 12;
/// Maximum transcript characters, before the total UTF-8 byte cap.
pub const TRANSCRIPT_CHARS: usize = 9_000;
/// Snapshot bytes, including metadata, role labels and truncation marks.
pub const SNAPSHOT_BYTES: usize = 24 * 1024;
/// Complete prompt bytes, including instructions and the question.
pub const PROMPT_BYTES: usize = 48 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::{
        AfterRun, Cadence, CatchUp, CreatePrompt, CreateSchedule, ScheduleDefinition,
        ScheduleTarget, UpdatePrompt, UpdateSchedule,
    };
    use crate::{
        Call, CreateSession, HarnessKind, ProtocolVersion, SavedPromptId, ScheduleId, SessionMode,
        SessionSpec,
    };

    /// The version side conversations came in; calls added later carry
    /// their own number, so this is not `PROTOCOL_VERSION`.
    const SIDE_QUESTIONS: ProtocolVersion = ProtocolVersion::new(19);

    #[test]
    fn every_direct_and_nested_side_spec_requires_protocol_19() {
        let parent = SessionId::random();
        let mut spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        );
        spec.side_question_of = Some(parent);
        let question: PromptText = "Why is this running?".parse().unwrap();
        let definition = ScheduleDefinition {
            name: None,
            cadence: Cadence::Every {
                seconds: 60_u32.try_into().unwrap(),
                anchor: None,
            },
            target: ScheduleTarget::NewSession {
                spec: Box::new(spec.clone()),
                after: AfterRun::default(),
            },
            prompt: question.clone(),
            catch_up: CatchUp::default(),
            max_runs: None,
            ends_at: None,
        };
        let calls = [
            Call::CreateSession(CreateSession { spec: spec.clone() }),
            Call::OpenSideQuestion(OpenSideQuestion { session: parent }),
            Call::PromptSideQuestion(PromptSideQuestion {
                session: SessionId::random(),
                question: question.clone(),
            }),
            Call::ResetSideQuestion(ResetSideQuestion {
                session: parent,
                side: SessionId::random(),
            }),
            Call::CreatePrompt(CreatePrompt {
                name: "Side".parse().unwrap(),
                text: question.clone(),
                project: None,
                session: Some(Box::new(spec.clone())),
            }),
            Call::UpdatePrompt(UpdatePrompt {
                prompt: SavedPromptId::random(),
                revision: 0,
                name: "Side".parse().unwrap(),
                text: question,
                session: Some(Box::new(spec)),
            }),
            Call::CreateSchedule(CreateSchedule {
                definition: definition.clone(),
                paused: false,
            }),
            Call::UpdateSchedule(UpdateSchedule {
                schedule: ScheduleId::random(),
                revision: 0,
                definition,
            }),
        ];
        for call in calls {
            assert_eq!(call.since(), Some(SIDE_QUESTIONS));
            let encoded = serde_json::to_string(&call).unwrap();
            let decoded: Call = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, call);
            let origin = crate::Origin {
                node: crate::NodeId::random(),
                by: crate::OriginBy::Owner,
                scopes: vec![crate::Scope::Read, crate::Scope::Operate],
            };
            let envelopes = [
                Call::Forward(crate::Forward {
                    node: crate::NodeId::random(),
                    call: Box::new(call.clone()),
                }),
                Call::Relay(crate::RelayCall {
                    node: crate::NodeId::random(),
                    origin: origin.clone(),
                    call: Box::new(call.clone()),
                }),
                Call::Deliver(crate::Deliver {
                    origin,
                    call: Box::new(call),
                }),
            ];
            for envelope in envelopes {
                assert_eq!(envelope.since(), Some(SIDE_QUESTIONS));
            }
        }
    }
}
