//! What waits in a session as the node keeps it now
//! (`session.conversation`: its permission requests and questions),
//! against what the log read so far holds.
//!
//! [Decision Log]
//! - Purpose: a request or a question whose record the node no longer
//!   keeps (its log pruned before the first record a client reads) still
//!   waits on the agent, and the node's live state still holds it whole,
//!   options and fields; the inbox (track N1) opens such a session and
//!   must answer it there.
//! - Constraint: the cards drew only what the log folded, so a pruned
//!   request had no card and the turn waited with no way to answer.
//! - Choice: the card draws a live request the log does not hold at all
//!   ([`Timeline::unseen_permissions`], [`Timeline::unseen_questions`]):
//!   nothing is added to the timeline (no entry is made up for a record
//!   that is gone), and one the log holds, waiting or settled, is the
//!   log's.
//! - Cost: the live state is a read of its own, taken when the session
//!   opens and after an answer goes; a request settled elsewhere while
//!   its record is gone shows until the next read.

use vessel_proto::conversation::Conversation;
use vessel_proto::{PermissionRequest, Question};

use super::fold::Timeline;

/// What waits in a session now, by the node's word.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Live {
    /// The permission requests waiting for an answer.
    pub permissions: Vec<PermissionRequest>,
    /// The questions waiting for an answer.
    pub questions: Vec<Question>,
}

impl Live {
    /// What `conversation` says waits.
    #[must_use]
    pub fn of(conversation: &Conversation) -> Self {
        Self {
            permissions: conversation.permissions.clone(),
            questions: conversation.questions.clone(),
        }
    }
}

impl Timeline {
    /// The requests of `live` the log read holds no record of, in the
    /// node's order.
    #[must_use]
    pub fn unseen_permissions<'a>(&self, live: &'a Live) -> Vec<&'a PermissionRequest> {
        live.permissions
            .iter()
            .filter(|request| !self.permissions.contains_key(&request.id))
            .collect()
    }

    /// The questions of `live` the log read holds no record of, in the
    /// node's order.
    #[must_use]
    pub fn unseen_questions<'a>(&self, live: &'a Live) -> Vec<&'a Question> {
        live.questions
            .iter()
            .filter(|question| !self.questions.contains_key(&question.id))
            .collect()
    }
}
