//! What a record says, for the index: a whole prompt, a chunk of a run, the
//! end of a run, or nothing the index reads.
//!
//! The rule lives here alone, so the writer's fast path and its catch-up
//! from the log read records the same way.

use vessel_proto::search::MessageRole;
use vessel_proto::{AgentEvent, ContentBlock, SessionEvent};

/// What one record means to the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Said<'a> {
    /// A message in one record (`turn_started`, `steered`): it ends any
    /// run, and is a piece (or pieces) of its own.
    Prompt(&'a [ContentBlock]),
    /// A chunk of a run of one role (`user_message_chunk`,
    /// `message_chunk`). A chunk without text still belongs to its run.
    Chunk(MessageRole, &'a ContentBlock),
    /// A timeline record that is not a chunk: whatever run is open ends.
    Ends,
    /// Not part of the conversation's timeline: a run goes on across it.
    Nothing,
}

/// Read one record.
pub(super) fn said(event: &SessionEvent) -> Said<'_> {
    match event {
        SessionEvent::Agent { event } if event.prompt_provenance().is_some() => Said::Ends,
        SessionEvent::Agent { event } => match event {
            AgentEvent::TurnStarted { content, .. } | AgentEvent::Steered { content, .. } => {
                Said::Prompt(content)
            }
            AgentEvent::UserMessageChunk { content, .. } => {
                Said::Chunk(MessageRole::Prompt, content)
            }
            AgentEvent::MessageChunk { content } => Said::Chunk(MessageRole::Reply, content),
            // A line on stderr can land between two chunks of one reply.
            AgentEvent::HarnessLog { .. }
            | AgentEvent::Notification(_)
            | AgentEvent::Compaction(_)
            | AgentEvent::Plugin(_)
            | AgentEvent::CommandAccepted { .. }
            | AgentEvent::Error { .. } => Said::Nothing,
            AgentEvent::Ready { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. }
            | AgentEvent::PermissionRequested(_)
            | AgentEvent::PermissionResolved { .. }
            | AgentEvent::TurnEnded { .. }
            | AgentEvent::TurnFailed { .. }
            | AgentEvent::QuestionAsked(_)
            | AgentEvent::QuestionResolved { .. } => Said::Ends,
        },
        SessionEvent::Status { .. } => Said::Ends,
        SessionEvent::Terminal(_)
        | SessionEvent::ProfileChanged { .. }
        | SessionEvent::ModelCall(_)
        | SessionEvent::OptionsChanged { .. }
        | SessionEvent::Renamed { .. }
        | SessionEvent::QueueChanged { .. }
        | SessionEvent::CommandsChanged { .. }
        | SessionEvent::UsageChanged { .. }
        | SessionEvent::Parked { .. }
        | SessionEvent::Woken
        | SessionEvent::Forked { .. }
        | SessionEvent::Rewound { .. } => Said::Nothing,
    }
}

/// The text a block adds to a message: a text block's text; nothing for a
/// link, an attachment or a block the node does not read.
pub(super) fn text_of(block: &ContentBlock) -> Option<&str> {
    match block {
        ContentBlock::Text { text } => Some(text),
        ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. }
        | ContentBlock::Attachment { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn local_notice_words_end_the_previous_run_without_becoming_user_search_text() {
        for value in ["turn_started", "steered", "user_message_chunk"] {
            let content = if value == "user_message_chunk" {
                serde_json::json!({"type":"text","text":"private notice"})
            } else {
                serde_json::json!([{"type":"text","text":"private notice"}])
            };
            let event = serde_json::from_value(serde_json::json!({
                "event": value, "content": content,
                "provenance": {"kind":"internal_notice","notice":5}
            }))
            .unwrap();
            assert_eq!(said(&SessionEvent::Agent { event }), Said::Ends);
        }
    }

    use super::*;
    use vessel_proto::{SessionStatus, StopReason};

    fn text(text: &str) -> ContentBlock {
        ContentBlock::Text {
            text: text.to_owned(),
        }
    }

    #[test]
    fn prompts_chunks_and_what_ends_a_run() {
        let prompt = SessionEvent::Agent {
            event: AgentEvent::TurnStarted {
                provenance: None,
                content: vec![text("hello")],

                client_message_id: None,
            },
        };
        assert!(matches!(said(&prompt), Said::Prompt(blocks) if blocks.len() == 1));
        let chunk = SessionEvent::Agent {
            event: AgentEvent::MessageChunk {
                content: text("hi"),
            },
        };
        assert!(matches!(said(&chunk), Said::Chunk(MessageRole::Reply, _)));
        let replayed = SessionEvent::Agent {
            event: AgentEvent::UserMessageChunk {
                provenance: None,
                content: text("before"),
            },
        };
        assert!(matches!(
            said(&replayed),
            Said::Chunk(MessageRole::Prompt, _)
        ));
        let end = SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: None,
                point: None,
            },
        };
        assert_eq!(said(&end), Said::Ends);
        let status = SessionEvent::Status {
            status: SessionStatus::Interrupted,
        };
        assert_eq!(said(&status), Said::Ends);
        let log = SessionEvent::Agent {
            event: AgentEvent::HarnessLog {
                line: "warn".to_owned(),
            },
        };
        assert_eq!(said(&log), Said::Nothing);
    }
}
