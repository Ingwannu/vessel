//! Where the agent's messages stand (protocol 12, ADR 0052): a turn's
//! point, and the check of a fork's replay.
//!
//! [Decision Log]
//! - Purpose: a fork or a rewind starts at a turn's end, named by the
//!   turn's last agent message (ACP's `messageId`), and an agent may ignore
//!   the anchor that names it; Vessel checks where a fork's replay ends.
//! - Choice: one tracker over the raw updates, before they are mapped
//!   (the mapped events carry no message id, so records keep their size).
//!   A point is the last agent message's id when nothing but the turn's
//!   end followed it: a thought, a plan, a tool call or its update after it
//!   would be cut off by a fork there (Claude's `TodoWrite` arrives as a
//!   plan). A chunk without an id continues the message before it; an id
//!   longer than `AgentMessageId::MAX_BYTES` is no point.
//! - The check: the fork's replay must end with the anchor's message and
//!   no user message may follow it (a head fork of a conversation whose
//!   later turns had no agent message would otherwise pass).

use agent_client_protocol::v1::schema as acp;
use vessel_proto::AgentMessageId;

/// The last agent message and what followed it.
#[derive(Debug, Default)]
pub(super) struct Tail {
    last: Option<AgentMessageId>,
    /// Nothing but more of that message followed it.
    clean: bool,
    /// A user message followed it.
    user_after: bool,
}

impl Tail {
    /// A new turn starts: nothing of it is seen yet.
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    /// One update, before it is mapped.
    pub(super) fn observe(&mut self, update: &acp::SessionUpdate) {
        use acp::SessionUpdate as U;
        match update {
            U::AgentMessageChunk(chunk) => {
                if let Some(id) = &chunk.message_id {
                    let id = id.0.as_ref();
                    if id.len() <= AgentMessageId::MAX_BYTES {
                        self.last = Some(AgentMessageId::new(id));
                        self.clean = true;
                    } else {
                        self.last = None;
                        self.clean = false;
                    }
                    self.user_after = false;
                }
            }
            U::UserMessageChunk(_) => {
                self.clean = false;
                self.user_after = true;
            }
            U::AgentThoughtChunk(_) | U::ToolCall(_) | U::ToolCallUpdate(_) | U::Plan(_) => {
                self.clean = false;
            }
            // Reports about the session, not its messages; an update kind
            // newer than the pinned ACP says nothing about a point either.
            U::AvailableCommandsUpdate(_)
            | U::CurrentModeUpdate(_)
            | U::ConfigOptionUpdate(_)
            | U::SessionInfoUpdate(_)
            | U::UsageUpdate(_)
            | U::Notice(_)
            | U::CompactionUpdate(_)
            | U::CompactionSummaryChunk(_)
            | _ => {}
        }
    }

    /// The turn's point, when it has one.
    pub(super) fn point(&self) -> Option<AgentMessageId> {
        self.last.clone().filter(|_| self.clean)
    }

    /// Whether a fork's replay ended with `anchor`'s message and nothing a
    /// person said after it.
    pub(super) fn ends_with(&self, anchor: &AgentMessageId) -> bool {
        self.last.as_ref() == Some(anchor) && !self.user_after
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(id: Option<&str>) -> acp::SessionUpdate {
        let mut chunk = acp::ContentChunk::new(acp::ContentBlock::from("text"));
        chunk.message_id = id.map(acp::MessageId::new);
        acp::SessionUpdate::AgentMessageChunk(chunk)
    }

    #[test]
    fn a_point_is_a_last_message_nothing_followed() {
        let mut tail = Tail::default();
        tail.observe(&message(Some("m1")));
        tail.observe(&message(None));
        assert_eq!(tail.point(), Some(AgentMessageId::new("m1")));
        tail.observe(&acp::SessionUpdate::AgentThoughtChunk(
            acp::ContentChunk::new(acp::ContentBlock::from("thinking")),
        ));
        assert_eq!(tail.point(), None, "a thought after it");
        tail.observe(&message(Some("m2")));
        assert_eq!(tail.point(), Some(AgentMessageId::new("m2")));
        tail.observe(&message(Some(&"x".repeat(AgentMessageId::MAX_BYTES + 1))));
        assert_eq!(tail.point(), None, "an id too long");
        tail.reset();
        tail.observe(&message(None));
        assert_eq!(tail.point(), None, "no id");
    }

    #[test]
    fn a_replay_ends_with_its_anchor_only_if_nobody_spoke_after_it() {
        let mut tail = Tail::default();
        tail.observe(&message(Some("m1")));
        assert!(tail.ends_with(&AgentMessageId::new("m1")));
        tail.observe(&acp::SessionUpdate::UserMessageChunk(
            acp::ContentChunk::new(acp::ContentBlock::from("next")),
        ));
        assert!(!tail.ends_with(&AgentMessageId::new("m1")));
    }
}
