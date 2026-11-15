//! Upload local input once; retain admitted source mutations if the CLI is stopped.
use super::CompareError;
use crate::conversation::file_input::FileInput;
use crate::error::CliError;
use std::{path::PathBuf, time::Duration};
use tokio::time::Instant;
use vessel_client::{NodeClient, Unary, comparison::MutationError};
use vessel_proto::conversation::{BeginUpload, FinishUpload, WriteUpload};
use vessel_proto::{
    AttachmentId, AttachmentKind, AttachmentName, AttachmentRef, NodeId, SessionId, UploadId,
};

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum State {
    #[default]
    Idle,
    Reading,
    InFlight,
    Acknowledged,
    Failed,
    Uncertain,
    Finished,
}
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Open,
    Read,
    Begin,
    Write,
    Finish,
}
#[derive(Debug, Default, serde::Serialize)]
pub(super) struct SourceTrail {
    pub(super) session: Option<SessionId>,
    file: Option<AttachmentName>,
    phase: Option<Phase>,
    state: State,
    upload: Option<UploadId>,
    acknowledged: u64,
    attempted: u64,
    observed_finish: Option<AttachmentId>,
    finished: Vec<AttachmentRef>,
}
impl SourceTrail {
    pub(super) fn interrupted(&mut self) {
        match self.state {
            State::InFlight => self.state = State::Uncertain,
            State::Reading => self.state = State::Failed,
            State::Idle
            | State::Acknowledged
            | State::Failed
            | State::Uncertain
            | State::Finished => {}
        }
    }
}
fn after(duration: Duration) -> Instant {
    Instant::now()
        .checked_add(duration)
        .unwrap_or_else(Instant::now)
}
const READ: Duration = Duration::from_secs(45);
const WRITE: Duration = Duration::from_secs(120);

pub(super) async fn upload(
    client: &NodeClient,
    destination: NodeId,
    session: SessionId,
    paths: &[PathBuf],
    trail: &mut SourceTrail,
) -> Result<Vec<AttachmentRef>, CliError> {
    trail.session = Some(session);
    let end = after(Duration::from_secs(1800));
    for path in paths {
        trail.phase = Some(Phase::Open);
        trail.state = State::Reading;
        trail.file = None;
        trail.upload = None;
        trail.observed_finish = None;
        trail.acknowledged = 0;
        trail.attempted = 0;
        let mut file = tokio::time::timeout_at(end.min(after(READ)), FileInput::open(path))
            .await
            .map_err(|_deadline| CompareError::SourceDeadline)??;
        trail.file = Some(file.name.clone());
        let begin = BeginUpload {
            session,
            name: file.name.clone(),
            mime: file.mime.clone(),
            size: file.size,
        };
        let upload = call(client, destination, begin, Phase::Begin, end, trail).await?;
        trail.upload = Some(upload.upload);
        if upload.chunk == 0 {
            trail.state = State::Failed;
            return Err(CompareError::SourceMismatch.into());
        }
        let mut progress = after(WRITE);
        loop {
            trail.phase = Some(Phase::Read);
            trail.state = State::Reading;
            let piece = tokio::time::timeout_at(
                end.min(progress).min(after(READ)),
                file.next(upload.chunk),
            )
            .await
            .map_err(|_deadline| CompareError::SourceDeadline)??;
            let Some(bytes) = piece else {
                break;
            };
            trail.attempted = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            let expected = trail
                .acknowledged
                .checked_add(trail.attempted)
                .ok_or(CompareError::SourceMismatch)?;
            let written = call(
                client,
                destination,
                WriteUpload {
                    upload: upload.upload,
                    offset: trail.acknowledged,
                    data: bytes,
                },
                Phase::Write,
                end.min(progress),
                trail,
            )
            .await?;
            if written.received != expected {
                trail.state = State::Failed;
                return Err(CompareError::SourceMismatch.into());
            }
            trail.acknowledged = written.received;
            progress = after(WRITE);
        }
        trail.attempted = 0;
        let finished = call(
            client,
            destination,
            FinishUpload {
                upload: upload.upload,
            },
            Phase::Finish,
            end,
            trail,
        )
        .await?;
        trail.observed_finish = Some(finished.id);
        if finished.name != file.name
            || finished.mime != file.mime
            || finished.size != file.size
            || finished.kind != AttachmentKind::of(&file.mime)
        {
            trail.state = State::Failed;
            return Err(CompareError::SourceMismatch.into());
        }
        trail.finished.push(finished);
    }
    trail.state = State::Finished;
    Ok(trail.finished.clone())
}
async fn call<R: Unary>(
    client: &NodeClient,
    destination: NodeId,
    request: R,
    phase: Phase,
    end: Instant,
    trail: &mut SourceTrail,
) -> Result<R::Response, CliError> {
    trail.phase = Some(phase);
    if Instant::now() >= end {
        trail.state = State::Failed;
        return Err(CompareError::SourceBudget.into());
    }
    trail.state = State::InFlight;
    match tokio::time::timeout_at(end.min(after(WRITE)), client.call_on(destination, request)).await
    {
        Ok(Ok(reply)) => {
            trail.state = State::Acknowledged;
            Ok(reply)
        }
        Ok(Err(source)) => {
            trail.state = match MutationError::of(&source) {
                MutationError::Uncertain => State::Uncertain,
                MutationError::Refused => State::Failed,
            };
            Err(source.into())
        }
        Err(_deadline) => {
            trail.state = State::Uncertain;
            Err(CompareError::SourceDeadline.into())
        }
    }
}
