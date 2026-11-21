//! A prompt's attachments on their way to the agent (protocol 10, ADR
//! 0025): the checks a prompt passes before it is sent or queued, and its
//! attachments' bodies, loaded through the node right before it is sent.
//!
//! Must not: read a file (the node keeps them and reads them back), or
//! send a prompt whose attachments were not checked here first.
//!
//! [Decision Log]
//! - Purpose: the session's actor sends prompts, delivered ones and the
//!   queue's, and must never block on file I/O while it does; a queued
//!   prompt is sent long after it was checked, possibly after a restart
//!   and a start again, when the agent may take less than it did.
//! - Constraint: the node keeps the files and their records (the store),
//!   which this crate does not depend on (ADR 0001).
//! - Alternatives: the node loads the bytes before it hands a prompt over
//!   (a queued prompt would hold up to 25 MiB while it waits, 32 a
//!   session, and the queue's own sends would still need the node); the
//!   actor reading the files (file I/O and the read-back rules in a crate
//!   that knows no data directory); a trait object the node implements (a
//!   callback into the node's state, where a channel is this workspace's
//!   way for one task to ask another).
//! - Choice: the node runs one loader; the runtime holds a sender to it
//!   ([`AttachmentLoader`]) and an actor asks it right before it sends a
//!   prompt naming an attachment, through `pump`, so the agent's output
//!   keeps flowing while the node reads on its blocking pool. The actor
//!   first checks what it can alone ([`check`]: the per-prompt limits, a
//!   kind its agent said it does not take), so nothing is read for a
//!   prompt refused anyway; the agent's driver checks the kind again
//!   against its own handshake, which also covers a prompt sent before
//!   the agent is ready.
//! - Cost: a round trip to the node per prompt with attachments, and the
//!   prompt's inline bytes (at most 25 MiB, as base64) in memory while it
//!   is sent; a steer the agent hands back (`promptRequired`) loads them
//!   again for the turn it starts.

use std::collections::HashSet;

use tokio::sync::{mpsc, oneshot};
use vessel_agent::Attachments;
use vessel_proto::{
    AgentAbilities, AttachmentId, AttachmentKind, AttachmentLimits, AttachmentRef, ContentBlock,
    SessionId,
};

use crate::error::SessionError;
use crate::structured::StructuredActor;

/// Where structured sessions ask for their prompts' attachments: the
/// node's loader (protocol 10). Cheap to clone.
#[derive(Debug, Clone)]
pub struct AttachmentLoader(mpsc::Sender<LoadAttachments>);

impl AttachmentLoader {
    /// A loader and the requests it receives, at most `capacity` waiting
    /// (an actor that finds no room waits for it).
    #[must_use]
    pub fn channel(capacity: usize) -> (Self, mpsc::Receiver<LoadAttachments>) {
        let (requests, received) = mpsc::channel(capacity.max(1));
        (Self(requests), received)
    }
}

/// One prompt's attachments to load: what the loader receives.
#[derive(Debug)]
pub struct LoadAttachments {
    /// The session the prompt is for: each attachment must be its.
    pub session: SessionId,
    /// What the prompt names, checked ([`check`]), each once.
    pub attachments: Vec<AttachmentRef>,
    /// Their bodies, or why not.
    pub reply: oneshot::Sender<Result<Attachments, AttachmentError>>,
}

/// Why a prompt's attachments were refused, or could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AttachmentError {
    /// More attachments than a prompt takes.
    #[error("a prompt names at most {limit} attachments; this one names {count}")]
    TooMany {
        /// How many it names.
        count: usize,
        /// The limit.
        limit: u32,
    },
    /// More bytes of images and audio than a prompt carries.
    #[error("a prompt carries at most {limit} bytes of images and audio; this one {bytes}")]
    TooLarge {
        /// How many it carries.
        bytes: u64,
        /// The limit.
        limit: u64,
    },
    /// The same attachment named twice.
    #[error("attachment {0} is named twice in the prompt")]
    Twice(AttachmentId),
    /// A kind the session's agent does not take.
    #[error("this session's agent does not take {} attachments (attachment {id})", noun(*.kind))]
    NotTaken {
        /// The attachment.
        id: AttachmentId,
        /// Its kind.
        kind: AttachmentKind,
    },
    /// Not an attachment of this session.
    #[error("attachment {0} is not one of this session's")]
    NotKept(AttachmentId),
    /// Its file is not the one kept (the node's read-back refused it).
    #[error("{0}")]
    Changed(String),
    /// This runtime has no loader: attachments are not kept here.
    #[error("attachments are not kept by this node")]
    NoLoader,
    /// Loading failed (reading the file, or the loader went away).
    #[error("loading the prompt's attachments failed: {0}")]
    Failed(String),
    /// A context or review block past its bounds (ADR 0124).
    #[error(transparent)]
    Context(#[from] vessel_proto::prompt_context::ContextProblem),
}

/// An attachment kind in a sentence.
const fn noun(kind: AttachmentKind) -> &'static str {
    match kind {
        AttachmentKind::Image => "image",
        AttachmentKind::Audio => "audio",
        AttachmentKind::File => "file",
    }
}

/// The attachments `content` names, within one prompt's limits
/// ([`AttachmentLimits::DEFAULT`]: how many, how many bytes of images and
/// audio), each once, and, where the agent's `abilities` are known, of a
/// kind it takes (files always). The references are the node's records,
/// so their kinds and sizes are true.
pub(crate) fn check(
    content: &[ContentBlock],
    abilities: Option<AgentAbilities>,
) -> Result<Vec<AttachmentRef>, AttachmentError> {
    let limits = AttachmentLimits::DEFAULT;
    let named: Vec<&AttachmentRef> = content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Attachment { attachment } => Some(attachment),
            ContentBlock::Text { .. }
            | ContentBlock::ResourceLink { .. }
            | ContentBlock::Opaque { .. }
            | ContentBlock::Context(_)
            | ContentBlock::Review(_) => None,
        })
        .collect();
    if named.len() > usize::try_from(limits.per_prompt).unwrap_or(usize::MAX) {
        return Err(AttachmentError::TooMany {
            count: named.len(),
            limit: limits.per_prompt,
        });
    }
    let mut seen = HashSet::with_capacity(named.len());
    let mut inline = 0_u64;
    for attachment in &named {
        if !seen.insert(attachment.id) {
            return Err(AttachmentError::Twice(attachment.id));
        }
        if attachment.kind.is_inline() {
            // Saturating: a sum past `u64` is over the limit anyway.
            inline = inline.saturating_add(attachment.size);
        }
        let taken = abilities.is_none_or(|abilities| match attachment.kind {
            AttachmentKind::Image => abilities.image,
            AttachmentKind::Audio => abilities.audio,
            AttachmentKind::File => true,
        });
        if !taken {
            return Err(AttachmentError::NotTaken {
                id: attachment.id,
                kind: attachment.kind,
            });
        }
    }
    if inline > limits.inline_per_prompt {
        return Err(AttachmentError::TooLarge {
            bytes: inline,
            limit: limits.inline_per_prompt,
        });
    }
    Ok(named.into_iter().cloned().collect())
}

impl StructuredActor {
    /// What the agent takes, once it has said so (`None` while it
    /// starts: its driver checks then).
    fn takes(&self) -> Option<AgentAbilities> {
        self.ready.then(|| self.desk.abilities())
    }

    /// Check what `content` names before it waits in the queue.
    pub(crate) fn check_attachments(&self, content: &[ContentBlock]) -> Result<(), SessionError> {
        check(content, self.takes())
            .map(drop)
            .map_err(SessionError::from)
    }

    /// The bodies of what `content` names, checked, then loaded by the
    /// node while the agent's output keeps being published; none for a
    /// prompt without attachments.
    pub(crate) async fn load_attachments(
        &mut self,
        content: &[ContentBlock],
    ) -> Result<Attachments, SessionError> {
        let wanted = check(content, self.takes())?;
        if wanted.is_empty() {
            return Ok(Attachments::new());
        }
        let Some(loader) = self.loader.clone() else {
            return Err(AttachmentError::NoLoader.into());
        };
        let (reply, answer) = oneshot::channel();
        let request = LoadAttachments {
            session: self.life.out.session(),
            attachments: wanted,
            reply,
        };
        let gone = || AttachmentError::Failed("the node's loader is gone".to_owned());
        let bodies = self
            .pump(async move {
                loader.0.send(request).await.map_err(|_closed| gone())?;
                answer.await.map_err(|_dropped| gone())?
            })
            .await
            .map_err(|_stopped| SessionError::Ended)??;
        Ok(bodies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attached(kind: AttachmentKind, size: u64) -> ContentBlock {
        ContentBlock::Attachment {
            attachment: AttachmentRef {
                id: AttachmentId::random(),
                name: "a.bin".parse().unwrap(),
                kind,
                mime: match kind {
                    AttachmentKind::Image => "image/png",
                    AttachmentKind::Audio => "audio/wav",
                    AttachmentKind::File => "application/octet-stream",
                }
                .parse()
                .unwrap(),
                size,
            },
        }
    }

    const SEES: AgentAbilities = AgentAbilities {
        steer: false,
        resume: false,
        image: true,
        audio: false,
        fork: false,
    };

    #[test]
    fn a_prompt_names_at_most_sixteen_attachments_each_once() {
        let text = ContentBlock::Text {
            text: "look".to_owned(),
        };
        let mut content = vec![text];
        content.extend((0..16).map(|_| attached(AttachmentKind::File, 50 << 20)));
        assert_eq!(check(&content, Some(SEES)).unwrap().len(), 16);
        content.push(attached(AttachmentKind::File, 1));
        assert_eq!(
            check(&content, Some(SEES)),
            Err(AttachmentError::TooMany {
                count: 17,
                limit: 16
            })
        );
        let twice = attached(AttachmentKind::File, 1);
        let ContentBlock::Attachment { attachment } = &twice else {
            panic!("an attachment block")
        };
        let id = attachment.id;
        assert_eq!(
            check(&[twice.clone(), twice], None),
            Err(AttachmentError::Twice(id))
        );
    }

    #[test]
    fn images_and_audio_count_against_the_inline_limit_and_files_do_not() {
        let mut content: Vec<_> = (0..2)
            .map(|_| attached(AttachmentKind::Image, 10 << 20))
            .collect();
        content.push(attached(AttachmentKind::File, 50 << 20));
        assert!(check(&content, Some(SEES)).is_ok(), "20 MiB inline");
        content.push(attached(AttachmentKind::Audio, 6 << 20));
        assert_eq!(
            check(&content, None),
            Err(AttachmentError::TooLarge {
                bytes: 26 << 20,
                limit: 25 << 20
            })
        );
    }

    #[test]
    fn a_kind_the_agent_does_not_take_is_refused_once_it_has_said() {
        let audio = [attached(AttachmentKind::Audio, 4)];
        assert!(matches!(
            check(&audio, Some(SEES)),
            Err(AttachmentError::NotTaken {
                kind: AttachmentKind::Audio,
                ..
            })
        ));
        assert!(
            check(&audio, None).is_ok(),
            "not said yet: the driver checks"
        );
        let blind = AgentAbilities {
            image: false,
            ..SEES
        };
        assert!(check(&[attached(AttachmentKind::File, 4)], Some(blind)).is_ok());
        assert!(check(&[attached(AttachmentKind::Image, 4)], Some(blind)).is_err());
    }
}
