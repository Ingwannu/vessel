//! Bounded stored-byte copying; only acknowledged writes reset progress time.
use super::endpoint::{Endpoint, MUTATE, READ};
use super::worker::{Entry, Reports};
use super::{ComparisonInput, CopyProgress, Failure, FailureKind, Phase, TargetStatus};
use std::time::Duration;
use tokio::time::Instant;
use vessel_proto::conversation::{
    BeginUpload, FinishUpload, PromptSession, ReadAttachment, WriteUpload,
};
use vessel_proto::{AttachmentRef, BlobChunk, ContentBlock, Delivered, Delivery};

pub(super) async fn dispatch(
    endpoint: &mut Endpoint,
    entry: &mut Entry,
    input: &ComparisonInput,
    reports: &Reports,
) {
    if !matches!(entry.report.status, TargetStatus::Ready) {
        return;
    }
    if let Err(failure) = super::validate::target(endpoint, entry, input).await {
        entry.failed(failure, false, reports);
        return;
    }
    let Some(session) = entry.report.resources.session else {
        return;
    };
    let end = super::endpoint::after(Duration::from_secs(1800));
    if let Some(source) = &input.source {
        for (index, file) in source.files.iter().enumerate() {
            let result = if source.session == session {
                entry.report.resources.copy = Some(CopyProgress {
                    source: source.session,
                    attachment: file.id,
                    attachment_index: index,
                    upload: None,
                    finished: None,
                    acknowledged: 0,
                    attempted_offset: 0,
                    attempted_bytes: 0,
                });
                entry.admitted(Phase::AttachmentRead, reports);
                // Verify even the source target before prompting: deletion/retention
                // must fail rather than silently substitute a later local file.
                verify(endpoint, source.session, file, end)
                    .await
                    .map(|()| file.clone())
            } else {
                copy(endpoint, entry, source.session, file, index, end, reports).await
            };
            match result {
                Ok(file) => entry.report.resources.attachments.push(file),
                Err(failure) => {
                    let mutation = matches!(
                        failure.phase,
                        Phase::UploadBegin | Phase::UploadWrite | Phase::UploadFinish
                    );
                    entry.failed(failure, mutation, reports);
                    return;
                }
            }
        }
    }
    let mut content =
        Vec::with_capacity(entry.report.resources.attachments.len().saturating_add(1));
    if let Some(text) = &input.text {
        content.push(ContentBlock::Text {
            text: text.as_str().to_owned(),
        });
    }
    content.extend(
        entry
            .report
            .resources
            .attachments
            .iter()
            .cloned()
            .map(|attachment| ContentBlock::Attachment { attachment }),
    );
    entry.admitted(Phase::Prompt, reports);
    match endpoint
        .call(
            Phase::Prompt,
            PromptSession {
                session,
                content,
                delivery: Delivery::Start,
                notify: false,
            },
            super::endpoint::after(MUTATE),
        )
        .await
    {
        Ok(Delivered::Started) => {
            entry.report.status = TargetStatus::Started;
            entry.publish(reports);
        }
        // A non-Start acknowledgement contradicts our admitted operation. It may
        // have delivered content, so report uncertainty rather than a safe retry.
        Ok(Delivered::Steered | Delivered::Queued { .. }) => {
            let failure = Failure::new(Phase::Prompt, FailureKind::Copy);
            entry.report.status = TargetStatus::Uncertain(failure);
            entry.publish(reports);
        }
        Err(failure) => entry.failed(failure, true, reports),
    }
}
async fn verify(
    endpoint: &mut Endpoint,
    source: vessel_proto::SessionId,
    file: &AttachmentRef,
    end: Instant,
) -> Result<(), Failure> {
    let deadline = deadline(Phase::AttachmentRead, end, READ)?;
    let chunk = endpoint
        .call(
            Phase::AttachmentRead,
            ReadAttachment {
                session: source,
                attachment: file.id,
                offset: 0,
            },
            deadline,
        )
        .await?;
    valid_chunk(&chunk, file.size, 0)
}
async fn copy(
    endpoint: &mut Endpoint,
    entry: &mut Entry,
    source: vessel_proto::SessionId,
    file: &AttachmentRef,
    index: usize,
    end: Instant,
    reports: &Reports,
) -> Result<AttachmentRef, Failure> {
    let session = entry
        .report
        .resources
        .session
        .ok_or_else(|| Failure::new(Phase::UploadBegin, FailureKind::Source))?;
    entry.report.resources.copy = Some(CopyProgress {
        source,
        attachment: file.id,
        attachment_index: index,
        upload: None,
        finished: None,
        acknowledged: 0,
        attempted_offset: 0,
        attempted_bytes: 0,
    });
    let begin_deadline = deadline(Phase::UploadBegin, end, MUTATE)?;
    entry.admitted(Phase::UploadBegin, reports);
    let upload = endpoint
        .call(
            Phase::UploadBegin,
            BeginUpload {
                session,
                name: file.name.clone(),
                mime: file.mime.clone(),
                size: file.size,
            },
            begin_deadline,
        )
        .await?;
    if let Some(progress) = &mut entry.report.resources.copy {
        progress.upload = Some(upload.upload);
    }
    entry.publish(reports);
    if upload.chunk == 0 {
        return Err(Failure::new(Phase::UploadBegin, FailureKind::Copy));
    }
    let accepted = stream(endpoint, entry, source, file, upload, end, reports).await?;
    let finish_deadline = deadline(Phase::UploadFinish, end, MUTATE)?;
    trail(entry, accepted, 0);
    entry.admitted(Phase::UploadFinish, reports);
    let finished = endpoint
        .call(
            Phase::UploadFinish,
            FinishUpload {
                upload: upload.upload,
            },
            finish_deadline,
        )
        .await?;
    // The response can identify a real stored file even when its metadata is
    // inconsistent. Keep that recovery identity before rejecting the response.
    if let Some(progress) = &mut entry.report.resources.copy {
        progress.finished = Some(finished.id);
    }
    entry.publish(reports);
    if finished.name != file.name
        || finished.kind != file.kind
        || finished.mime != file.mime
        || finished.size != file.size
    {
        return Err(Failure::new(Phase::UploadFinish, FailureKind::Copy));
    }
    Ok(finished)
}
async fn stream(
    endpoint: &mut Endpoint,
    entry: &mut Entry,
    source: vessel_proto::SessionId,
    file: &AttachmentRef,
    upload: vessel_proto::conversation::Upload,
    end: Instant,
    reports: &Reports,
) -> Result<u64, Failure> {
    let mut accepted = 0_u64;
    let mut progress_end = super::endpoint::after(MUTATE);
    let mut first = true;
    while accepted < file.size || first {
        first = false;
        let read_deadline = deadline(Phase::AttachmentRead, end.min(progress_end), READ)?;
        trail(entry, accepted, 0);
        entry.admitted(Phase::AttachmentRead, reports);
        let chunk = endpoint
            .call(
                Phase::AttachmentRead,
                ReadAttachment {
                    session: source,
                    attachment: file.id,
                    offset: accepted,
                },
                read_deadline,
            )
            .await?;
        valid_chunk(&chunk, file.size, accepted)?;
        let width = usize::try_from(upload.chunk)
            .map_err(|source| Failure::count(Phase::UploadWrite, source))?;
        let mut from = 0_usize;
        while from < chunk.data.len() {
            let to = from.saturating_add(width).min(chunk.data.len());
            let piece = chunk.data.slice(from..to);
            let length = u64::try_from(piece.len())
                .map_err(|source| Failure::count(Phase::UploadWrite, source))?;
            let expected = accepted
                .checked_add(length)
                .ok_or_else(|| Failure::new(Phase::UploadWrite, FailureKind::Copy))?;
            let write_deadline = deadline(Phase::UploadWrite, end.min(progress_end), MUTATE)?;
            trail(entry, accepted, length);
            entry.admitted(Phase::UploadWrite, reports);
            let written = endpoint
                .call(
                    Phase::UploadWrite,
                    WriteUpload {
                        upload: upload.upload,
                        offset: accepted,
                        data: piece,
                    },
                    write_deadline,
                )
                .await?;
            if written.received != expected {
                return Err(Failure::new(Phase::UploadWrite, FailureKind::Copy));
            }
            accepted = written.received;
            if let Some(progress) = &mut entry.report.resources.copy {
                progress.acknowledged = accepted;
            }
            entry.publish(reports);
            // Only positive destination acceptance resets this watchdog. Reads
            // and metadata replies can never keep a stalled copy alive.
            progress_end = super::endpoint::after(MUTATE);
            from = to;
        }
    }
    Ok(accepted)
}

fn trail(entry: &mut Entry, offset: u64, bytes: u64) {
    if let Some(progress) = &mut entry.report.resources.copy {
        progress.attempted_offset = offset;
        progress.attempted_bytes = bytes;
    }
}
fn deadline(phase: Phase, end: Instant, operation: Duration) -> Result<Instant, Failure> {
    let now = Instant::now();
    if now >= end {
        return Err(Failure::new(phase, FailureKind::CopyBudget));
    }
    Ok(end.min(super::endpoint::after(operation)))
}
fn valid_chunk(chunk: &BlobChunk, size: u64, offset: u64) -> Result<(), Failure> {
    let length = u64::try_from(chunk.data.len())
        .map_err(|source| Failure::count(Phase::AttachmentRead, source))?;
    if chunk.total != size
        || chunk.data.len() > BlobChunk::MAX_LEN
        || (length == 0 && offset < size)
        || offset.checked_add(length).is_none_or(|end| end > size)
    {
        return Err(Failure::new(Phase::AttachmentRead, FailureKind::Copy));
    }
    Ok(())
}
