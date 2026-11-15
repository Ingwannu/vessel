//! Agent variant admission and independent optional-field/content masks.
use crate::{AgentEvent, ContentBlock, ProtocolVersion, QuestionOutcome};

impl AgentEvent {
    /// The first protocol version that can decode this event: its
    /// variant's, or 10 for a prompt holding an attachment. A feature
    /// awaiting its assigned protocol returns `None`; it must never be
    /// published as though it belonged to the current protocol.
    #[must_use]
    pub fn required_protocol(&self) -> Option<ProtocolVersion> {
        match self {
            Self::Notification(_)
            | Self::Compaction(_)
            | Self::Plugin(_)
            | Self::CommandAccepted { .. }
            | Self::Error { .. } => crate::timeline::TIMELINE_PARITY,
            Self::Steered { .. } | Self::QuestionAsked(_) | Self::QuestionResolved { .. } => {
                Some(ProtocolVersion::new(10))
            }
            Self::TurnStarted { content, .. } if content.iter().any(is_attachment) => {
                Some(ProtocolVersion::new(10))
            }
            Self::UserMessageChunk { content } if is_attachment(content) => {
                Some(ProtocolVersion::new(10))
            }
            Self::Ready { .. }
            | Self::TurnStarted { .. }
            | Self::UserMessageChunk { .. }
            | Self::MessageChunk { .. }
            | Self::ThoughtChunk { .. }
            | Self::ToolCall(_)
            | Self::ToolCallUpdate(_)
            | Self::Plan { .. }
            | Self::PermissionRequested(_)
            | Self::PermissionResolved { .. }
            | Self::TurnEnded { .. }
            | Self::TurnFailed { .. }
            | Self::HarnessLog { .. } => Some(ProtocolVersion::new(2)),
        }
    }

    /// What a client speaking `protocol` receives instead, or `None` when
    /// it can read this event as it is.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        self.for_protocol_with_reference(protocol, crate::timeline::ReferenceContext::CANONICAL)
    }

    pub(crate) fn for_protocol_with_reference(
        &self,
        protocol: ProtocolVersion,
        context: crate::timeline::ReferenceContext,
    ) -> Option<Self> {
        if let Some(adapted) =
            crate::timeline::compat::event_with_reference(self, protocol, context)
        {
            // Optional fields and a newer content block can need independent masks.
            let legacy = adapted.base_for_protocol(protocol);
            return Some(legacy.unwrap_or(adapted));
        }
        self.base_for_protocol(protocol)
    }

    fn base_for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        if self
            .required_protocol()
            .is_some_and(|since| since <= protocol)
        {
            return None;
        }
        let log = |line: String| Some(Self::HarnessLog { line });
        match self {
            Self::CommandAccepted { .. } => log("vessel: native command accepted".to_owned()),
            Self::Notification(notice) => {
                log(format!("vessel: {:?}: {}", notice.level, notice.message))
            }
            Self::Compaction(compaction) => log(format!(
                "vessel: context compaction {:?}",
                compaction.status
            )),
            Self::Plugin(item) => log(format!(
                "vessel: plugin {} contribution {}",
                item.plugin, item.id
            )),
            Self::Error { message } => log(format!("vessel: error: {message}")),
            Self::Steered { content, .. } => log(match first_line(content) {
                Some(line) => format!("vessel: steered: {line}"),
                None => "vessel: steered".to_owned(),
            }),
            Self::QuestionAsked(question) => log(format!(
                "vessel: the agent asks: {}",
                question.message.lines().next().unwrap_or_default()
            )),
            Self::QuestionResolved { outcome, .. } => log(match outcome {
                QuestionOutcome::Answered { .. } => "vessel: question answered",
                QuestionOutcome::Declined => "vessel: question declined",
                QuestionOutcome::Cancelled => "vessel: question cancelled",
            }
            .to_owned()),
            Self::TurnStarted { content, .. } => Some(Self::TurnStarted {
                content: content.iter().map(link_for_attachment).collect(),

                client_message_id: None,
            }),
            Self::UserMessageChunk { content } => Some(Self::UserMessageChunk {
                content: link_for_attachment(content),
            }),
            Self::Ready { .. }
            | Self::MessageChunk { .. }
            | Self::ThoughtChunk { .. }
            | Self::ToolCall(_)
            | Self::ToolCallUpdate(_)
            | Self::Plan { .. }
            | Self::PermissionRequested(_)
            | Self::PermissionResolved { .. }
            | Self::TurnEnded { .. }
            | Self::TurnFailed { .. }
            | Self::HarnessLog { .. } => None,
        }
    }
}

const fn is_attachment(block: &ContentBlock) -> bool {
    matches!(block, ContentBlock::Attachment { .. })
}

/// An attachment as a version 2-9 client can decode it: a link to
/// `attachment:<id>`, named after its file. Other blocks as they are.
fn link_for_attachment(block: &ContentBlock) -> ContentBlock {
    match block {
        ContentBlock::Attachment { attachment } => ContentBlock::ResourceLink {
            uri: format!("attachment:{}", attachment.id),
            name: attachment.name.to_string(),
        },
        ContentBlock::Text { .. }
        | ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. } => block.clone(),
    }
}

/// The first line of a message's first text, for a log line.
fn first_line(content: &[ContentBlock]) -> Option<&str> {
    content.iter().find_map(|block| match block {
        ContentBlock::Text { text } => text.lines().map(str::trim).find(|line| !line.is_empty()),
        ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. }
        | ContentBlock::Attachment { .. } => None,
    })
}
