//! A prompt's attachments on the node's side (protocol 10, ADR 0025):
//! the references a client sent replaced by the node's own records
//! ([`resolve`]), and the loader a session's actor asks for their bodies
//! right before it sends the prompt ([`serve`]).
//!
//! Must not: hand an agent a body read other than through the store's
//! read-back (no link, the recorded size and SHA-256), or keep a client's
//! name, kind, type or size for an attachment.
//!
//! [Decision Log]
//! - Purpose: a client could relabel an image as a file to pass the
//!   inline limit, or name another session's attachment; the node's
//!   record decides (the contract's review, item 9). The bytes are read
//!   on the blocking pool, never by the session's actor (see
//!   `vessel_session`'s `attachments`).
//! - Resolution happens where a prompt enters the node (`session.prompt`,
//!   a queue `edit`, `session.send`), so what waits in the queue, and what
//!   the store keeps of it, is the node's record; the loader looks each one
//!   up again, so an entry sent much later is still this session's.
//! - Images and audio go inline as base64, encoded on the blocking pool;
//!   a file is checked (hashed once per change, `Verified`) and handed as
//!   a `file:` URI to where the store keeps it.
//! - The loader is one task taking requests in order and answering each
//!   from a task of its own, so a session waiting for a large image does
//!   not hold up another's prompt; at most one request per session is in
//!   flight (its actor waits for the answer).

use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::{AttachmentKind, AttachmentLimits, AttachmentRef, ContentBlock, SessionId};
use vessel_session::{AttachmentBody, AttachmentError, Attachments, LoadAttachments};
use vessel_store::{AttachmentFileError, KeptAttachment, Store, StoreError};

use super::Verified;
use crate::error::ServiceError;

/// `content` with each attachment it names replaced by `session`'s record
/// of it; an attachment that is not `session`'s is refused. More than a
/// prompt takes is refused before anything is looked up.
pub(crate) async fn resolve(
    store: &Store,
    session: SessionId,
    content: Vec<ContentBlock>,
) -> Result<Vec<ContentBlock>, ServiceError> {
    let named = content
        .iter()
        .filter(|block| matches!(block, ContentBlock::Attachment { .. }))
        .count();
    if named == 0 {
        return Ok(content);
    }
    let limit = AttachmentLimits::DEFAULT.per_prompt;
    if named > usize::try_from(limit).unwrap_or(usize::MAX) {
        let refused = AttachmentError::TooMany {
            count: named,
            limit,
        };
        return Err(vessel_session::SessionError::from(refused).into());
    }
    let mut resolved = Vec::with_capacity(content.len());
    for block in content {
        resolved.push(match block {
            ContentBlock::Attachment { attachment } => ContentBlock::Attachment {
                attachment: store
                    .attachment(session, attachment.id)
                    .await?
                    .map(|kept| kept.reference)
                    .ok_or_else(|| {
                        vessel_session::SessionError::from(AttachmentError::NotKept(attachment.id))
                    })?,
            },
            other @ (ContentBlock::Text { .. }
            | ContentBlock::ResourceLink { .. }
            | ContentBlock::Opaque { .. }) => other,
        });
    }
    Ok(resolved)
}

/// Answer the sessions' load requests until `stop`.
pub(crate) fn serve(
    store: Store,
    verified: Arc<Verified>,
    mut requests: mpsc::Receiver<LoadAttachments>,
    tasks: &TaskTracker,
    stop: &CancellationToken,
) {
    let stop = stop.clone();
    let answering = tasks.clone();
    drop(tasks.spawn(async move {
        loop {
            let request = tokio::select! {
                () = stop.cancelled() => return,
                request = requests.recv() => match request {
                    Some(request) => request,
                    None => return,
                },
            };
            let (store, verified) = (store.clone(), Arc::clone(&verified));
            drop(answering.spawn(async move {
                let LoadAttachments {
                    session,
                    attachments,
                    reply,
                } = request;
                let loaded = load(&store, &verified, session, attachments).await;
                // The session may have stopped waiting (it ended).
                let _gone = reply.send(loaded);
            }));
        }
    }));
}

/// The bodies of `wanted`, each looked up again as `session`'s.
async fn load(
    store: &Store,
    verified: &Verified,
    session: SessionId,
    wanted: Vec<AttachmentRef>,
) -> Result<Attachments, AttachmentError> {
    let mut loaded = Attachments::new();
    for reference in wanted {
        let kept = store
            .attachment(session, reference.id)
            .await
            .map_err(|error| failed(&error))?
            .ok_or(AttachmentError::NotKept(reference.id))?;
        let id = kept.reference.id;
        let body = match kept.reference.kind {
            AttachmentKind::Image | AttachmentKind::Audio => inline(store, verified, kept).await?,
            AttachmentKind::File => link(store, verified, kept).await?,
        };
        loaded.insert(id, body);
    }
    Ok(loaded)
}

/// An image's or audio's bytes, read back whole, as base64.
async fn inline(
    store: &Store,
    verified: &Verified,
    kept: KeptAttachment,
) -> Result<AttachmentBody, AttachmentError> {
    let id = kept.reference.id;
    let size = usize::try_from(kept.reference.size).unwrap_or(usize::MAX);
    let read = store
        .read_attachment(kept, 0, size, verified.get(id))
        .await
        .map_err(|error| refused(&error))?;
    verified.put(id, read.identity);
    let data = read.data;
    let encoded = tokio::task::spawn_blocking(move || STANDARD.encode(data))
        .await
        .map_err(|error| AttachmentError::Failed(error.to_string()))?;
    Ok(AttachmentBody::Inline(encoded))
}

/// A file, checked, as a `file:` URI to where it is kept.
async fn link(
    store: &Store,
    verified: &Verified,
    kept: KeptAttachment,
) -> Result<AttachmentBody, AttachmentError> {
    let id = kept.reference.id;
    let path = store.attachment_path(&kept);
    let checked = store
        .read_attachment(kept, 0, 0, verified.get(id))
        .await
        .map_err(|error| refused(&error))?;
    verified.put(id, checked.identity);
    let uri = url::Url::from_file_path(&path).map_err(|()| {
        AttachmentError::Failed(format!(
            "{} is not an absolute path; give the node an absolute data directory",
            path.display()
        ))
    })?;
    Ok(AttachmentBody::Link(uri.into()))
}

/// A read-back refusal as the session reports it: a file that is not the
/// one kept is the node's state (`conflict`), anything else a failure.
fn refused(error: &StoreError) -> AttachmentError {
    if let StoreError::Attachment(
        refusal @ (AttachmentFileError::Changed { .. } | AttachmentFileError::Missing(_)),
    ) = error
    {
        return AttachmentError::Changed(refusal.to_string());
    }
    failed(error)
}

fn failed(error: &StoreError) -> AttachmentError {
    AttachmentError::Failed(crate::error::chain(error))
}
