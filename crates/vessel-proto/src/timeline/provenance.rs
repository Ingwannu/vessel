//! A local notice's presentation link is correlation, never input authority.
//! Producers attach it only after their one notice/queue/receipt transaction;
//! external provider replay is scrubbed before the actor can interpret it.

use crate::{AgentEvent, ContentBlock, EventSeq, QueuedPrompt};
use serde::{Deserialize, Serialize};

/// Why ordinary prompt words are represented by another canonical timeline item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PromptProvenance {
    /// The one locally committed notice remains visible at this actual sequence.
    InternalNotice {
        /// Canonical notification record; never a synthetic turn or new alias.
        notice: EventSeq,
    },
}

impl AgentEvent {
    /// The local presentation association, independent of real turn lifecycle.
    #[must_use]
    pub const fn prompt_provenance(&self) -> Option<PromptProvenance> {
        if let Self::TurnStarted { provenance, .. }
        | Self::Steered { provenance, .. }
        | Self::UserMessageChunk { provenance, .. } = self
        {
            *provenance
        } else {
            None
        }
    }

    /// Hide duplicate ordinary input words while preserving the typed notice
    /// link and the actual event/lifecycle. Reader adapters share this leaf;
    /// it performs no recursive compatibility traversal and grants no authority.
    pub fn clear_internal_notice_words(&mut self) {
        if self.prompt_provenance().is_none() {
            return;
        }
        if let Self::TurnStarted { content, .. } | Self::Steered { content, .. } = self {
            content.clear();
        } else if let Self::UserMessageChunk { content, .. } = self {
            *content = ContentBlock::Text {
                text: String::new(),
            };
        }
    }

    /// Foreign provider/history input cannot suppress ordinary user words by
    /// supplying a serialized local association. Canonical retained reads do not
    /// call this: their association was created by the local persistence owner.
    pub fn clear_foreign_prompt_provenance(&mut self) {
        if let Self::TurnStarted { provenance, .. }
        | Self::Steered { provenance, .. }
        | Self::UserMessageChunk { provenance, .. } = self
        {
            *provenance = None;
        }
    }
}

impl QueuedPrompt {
    /// The shared reader leaf suppresses internal words without losing queue
    /// identity, order, revision, creator or the canonical notice association.
    pub fn clear_internal_notice_words(&mut self) {
        if self.provenance.is_some() {
            self.content.clear();
        }
    }
}

#[cfg(test)]
mod tests;
