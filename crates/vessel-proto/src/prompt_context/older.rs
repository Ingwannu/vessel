//! A prompt's blocks as a peer of an older protocol decodes them: an
//! attachment as a link before 10 (ADR 0025), a `context` or `review`
//! block as a `text` block of what the agent read before
//! [`PROMPT_CONTEXT_PROTOCOL`] (ADR 0124), so its timeline and its queue
//! still show the words.

use super::{PROMPT_CONTEXT_PROTOCOL, agent_text};
use crate::ContentBlock;
use crate::conversation::{PromptQueue, QueuedPrompt};
use crate::node::ProtocolVersion;

/// The first version with attachment blocks.
const ATTACHMENTS: ProtocolVersion = ProtocolVersion::new(10);

/// Whether a peer speaking `protocol` decodes `context` and `review`.
fn speaks_context(protocol: ProtocolVersion) -> bool {
    PROMPT_CONTEXT_PROTOCOL.is_some_and(|since| since <= protocol)
}

const fn is_context(block: &ContentBlock) -> bool {
    matches!(block, ContentBlock::Context(_) | ContentBlock::Review(_))
}

/// The first version that decodes every block of `content`; `None` while
/// [`PROMPT_CONTEXT_PROTOCOL`] awaits its number and `content` holds one
/// of its blocks.
pub(crate) fn content_protocol(content: &[ContentBlock]) -> Option<ProtocolVersion> {
    if content.iter().any(is_context) {
        return PROMPT_CONTEXT_PROTOCOL;
    }
    if content
        .iter()
        .any(|block| matches!(block, ContentBlock::Attachment { .. }))
    {
        return Some(ATTACHMENTS);
    }
    Some(ProtocolVersion::new(2))
}

/// `block` as a peer speaking `protocol` decodes it.
pub(crate) fn block_for_protocol(block: &ContentBlock, protocol: ProtocolVersion) -> ContentBlock {
    match block {
        ContentBlock::Attachment { attachment } if protocol < ATTACHMENTS => {
            ContentBlock::ResourceLink {
                uri: format!("attachment:{}", attachment.id),
                name: attachment.name.to_string(),
            }
        }
        ContentBlock::Context(_) | ContentBlock::Review(_) if !speaks_context(protocol) => {
            ContentBlock::Text {
                text: agent_text(block).unwrap_or_default(),
            }
        }
        ContentBlock::Text { .. }
        | ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. }
        | ContentBlock::Attachment { .. }
        | ContentBlock::Context(_)
        | ContentBlock::Review(_) => block.clone(),
    }
}

impl PromptQueue {
    /// This queue as a peer speaking `protocol` reads it, or `None` when it
    /// reads it as it is.
    #[must_use]
    pub(crate) fn context_for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        if speaks_context(protocol)
            || !self
                .entries
                .iter()
                .any(|entry| entry.content.iter().any(is_context))
        {
            return None;
        }
        let entries = self
            .entries
            .iter()
            .map(|entry| QueuedPrompt {
                content: entry
                    .content
                    .iter()
                    .map(|block| block_for_protocol(block, protocol))
                    .collect(),
                ..entry.clone()
            })
            .collect();
        Some(Self {
            entries,
            ..self.clone()
        })
    }
}

/// A protocol awaiting its number.
pub(crate) struct Pending;

/// `Some(n)` when `content` holds a `context` or `review` block of
/// protocol n (ADR 0124), `None` when it holds none; `Pending` while that
/// protocol awaits its number.
pub(crate) fn context_since(content: &[ContentBlock]) -> Result<Option<u32>, Pending> {
    if content
        .iter()
        .any(|block| matches!(block, ContentBlock::Context(_) | ContentBlock::Review(_)))
    {
        return PROMPT_CONTEXT_PROTOCOL
            .map(|since| Some(since.get()))
            .ok_or(Pending);
    }
    Ok(None)
}
