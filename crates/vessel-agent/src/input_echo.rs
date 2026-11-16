//! Local accepted input evidence, never provider/history metadata or authority.
//! The closed constructors preserve the exact driver's typed input; session
//! publication supplies its canonical record and persistence watermark later.
use vessel_proto::timeline::{ClientMessageId, PromptProvenance};
use vessel_proto::{AgentEvent, ContentBlock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Started,
    Steered,
}

/// An accepted local input echo with no serde constructor or arbitrary event.
/// Its origin does not authorize submission, queue mutation or a permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEcho {
    kind: Kind,
    content: Vec<ContentBlock>,
    message: Option<ClientMessageId>,
    provenance: Option<PromptProvenance>,
}

impl InputEcho {
    pub(crate) fn started(
        content: Vec<ContentBlock>,
        message: Option<ClientMessageId>,
        provenance: Option<PromptProvenance>,
    ) -> Self {
        Self {
            kind: Kind::Started,
            content,
            message,
            provenance,
        }
    }

    pub(crate) fn steered(
        content: Vec<ContentBlock>,
        message: Option<ClientMessageId>,
        provenance: Option<PromptProvenance>,
    ) -> Self {
        Self {
            kind: Kind::Steered,
            content,
            message,
            provenance,
        }
    }

    /// Consume exactly this accepted input. The actor assigns the real sequence;
    /// foreign ordinary events still pass through its unconditional ingress scrub.
    #[must_use]
    pub fn into_event(self) -> AgentEvent {
        match self.kind {
            Kind::Started => AgentEvent::TurnStarted {
                content: self.content,
                client_message_id: self.message,
                provenance: self.provenance,
            },
            Kind::Steered => AgentEvent::Steered {
                content: self.content,
                client_message_id: self.message,
                provenance: self.provenance,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::EventSeq;

    #[test]
    fn the_closed_echo_preserves_the_exact_local_input_identity_and_origin() {
        let content = vec![ContentBlock::Text {
            text: "same visible words".into(),
        }];
        let message = ClientMessageId::random();
        let provenance = Some(PromptProvenance::InternalNotice {
            notice: EventSeq::new(17),
        });
        assert_eq!(
            InputEcho::started(content.clone(), Some(message.clone()), provenance).into_event(),
            AgentEvent::TurnStarted {
                content: content.clone(),
                client_message_id: Some(message.clone()),
                provenance
            },
        );
        assert_eq!(
            InputEcho::steered(content.clone(), Some(message.clone()), provenance).into_event(),
            AgentEvent::Steered {
                content: content.clone(),
                client_message_id: Some(message),
                provenance
            },
        );
        assert_eq!(
            InputEcho::started(content.clone(), None, None).into_event(),
            AgentEvent::TurnStarted {
                content,
                client_message_id: None,
                provenance: None
            },
        );
    }
}
