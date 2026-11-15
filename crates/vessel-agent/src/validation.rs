//! Pure provider-edge preflight before the node reserves an identified prompt.

use crate::{Attachments, Driver, InputError};
use vessel_proto::{AgentAbilities, AttachmentKind, ContentBlock};

/// Validate a prompt without provider, queue, or receipt effects.
/// Attachment identity/body resolution remains the owning node's responsibility.
///
/// # Errors
/// The same content/kind errors the selected driver's prompt decoder returns.
pub fn validate_prompt(
    driver: Driver,
    content: &[ContentBlock],
    abilities: AgentAbilities,
) -> Result<(), InputError> {
    match driver {
        Driver::CodexAppServer => crate::codex::validate_prompt(content),
        Driver::ClaudeStream => crate::claude::validate_prompt(content),
        Driver::OpencodeHttp => Err(InputError::Closed),
        Driver::Acp => {
            for block in content {
                match block {
                    ContentBlock::Attachment { attachment } => {
                        let supported = match attachment.kind {
                            AttachmentKind::Image => abilities.image,
                            AttachmentKind::Audio => abilities.audio,
                            AttachmentKind::File => true,
                        };
                        if !supported {
                            return Err(InputError::AttachmentNotTaken {
                                id: attachment.id,
                                kind: attachment.kind,
                            });
                        }
                    }
                    ContentBlock::Text { .. }
                    | ContentBlock::ResourceLink { .. }
                    | ContentBlock::Opaque { .. } => {
                        let _validated = crate::mapper::prompt_block(
                            block.clone(),
                            &mut Attachments::new(),
                            abilities,
                        )?;
                    }
                }
            }
            Ok(())
        }
    }
}
