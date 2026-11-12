//! The prompt side, Vessel to ACP: a prompt's blocks as the agent takes
//! them, an attachment's body put in (an image or audio inline, a file as
//! a link to the stored copy) where the agent said it takes that kind.

use agent_client_protocol::v1::schema as acp;
use vessel_proto::{AgentAbilities, AttachmentKind, AttachmentRef, ContentBlock};

use crate::attached::{AttachmentBody, Attachments};
use crate::error::InputError;

/// A prompt block in ACP form. Opaque blocks must be ACP content already
/// (they are what [`content`](super::content) forwarded), so they are parsed back. An
/// attachment takes its body from `attachments` (protocol 10): an image or
/// audio inline, a file as a link, each only if the agent said in its
/// handshake (`abilities`) that it takes that kind; files it always takes.
pub(crate) fn prompt_block(
    block: ContentBlock,
    attachments: &mut Attachments,
    abilities: AgentAbilities,
) -> Result<acp::ContentBlock, InputError> {
    match block {
        ContentBlock::Text { text } => Ok(acp::ContentBlock::Text(acp::TextContent::new(text))),
        ContentBlock::ResourceLink { uri, name } => Ok(acp::ContentBlock::ResourceLink(
            acp::ResourceLink::new(name, uri),
        )),
        ContentBlock::Opaque { raw } => {
            serde_json::from_str(raw.as_text()).map_err(InputError::InvalidContent)
        }
        ContentBlock::Attachment { attachment } => attached(attachment, attachments, abilities),
    }
}

/// An attachment as the agent takes it: an `image` or `audio` block with
/// its bytes, or a `resource_link` to its file.
pub(super) fn attached(
    attachment: AttachmentRef,
    attachments: &mut Attachments,
    abilities: AgentAbilities,
) -> Result<acp::ContentBlock, InputError> {
    let AttachmentRef {
        id,
        name,
        kind,
        mime,
        size,
    } = attachment;
    let taken = match kind {
        AttachmentKind::Image => abilities.image,
        AttachmentKind::Audio => abilities.audio,
        AttachmentKind::File => true,
    };
    if !taken {
        return Err(InputError::AttachmentNotTaken { id, kind });
    }
    let mime = mime.as_str().to_owned();
    match (kind, attachments.take(id)) {
        (AttachmentKind::Image, Some(AttachmentBody::Inline(data))) => {
            Ok(acp::ContentBlock::Image(acp::ImageContent::new(data, mime)))
        }
        (AttachmentKind::Audio, Some(AttachmentBody::Inline(data))) => {
            Ok(acp::ContentBlock::Audio(acp::AudioContent::new(data, mime)))
        }
        (AttachmentKind::File, Some(AttachmentBody::Link(uri))) => {
            Ok(acp::ContentBlock::ResourceLink(
                acp::ResourceLink::new(name.as_str(), uri)
                    .mime_type(mime)
                    .size(i64::try_from(size).ok()),
            ))
        }
        (
            AttachmentKind::Image | AttachmentKind::Audio | AttachmentKind::File,
            Some(AttachmentBody::Inline(_) | AttachmentBody::Link(_)) | None,
        ) => Err(InputError::AttachmentNotLoaded(id)),
    }
}
