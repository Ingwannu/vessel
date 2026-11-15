//! The upload task (`super`'s decision log says why it is shaped so).

use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use crate::connections::{RequestFailure, SelectedConnection};
use bytes::Bytes;
use tokio::sync::mpsc;
use vessel_app_view::composer::attach::looks_like;
use vessel_app_view::composer::tray::{Again, Begun, Origin, Problem};
use vessel_client::CallError;
use vessel_proto::{AttachmentRef, BlobChunk, ContentBlock, ErrorCode};

use super::{Answer, Attach, Step};
use crate::Event;

/// The bytes an image's type is checked by.
const HEAD: usize = 16;

/// Send `attach` beside the link's other commands, telling each step;
/// `client` is the machine's connection, or why there is none.
pub(crate) fn spawn(
    client: Result<SelectedConnection, crate::LinkError>,
    attach: Attach,
    events: mpsc::Sender<Event>,
) {
    drop(tokio::spawn(async move {
        let session = attach.session;
        let key = attach.key;
        let message = attach.message.clone();
        let tell = |step: Step| {
            let events = events.clone();
            let message = message.clone();
            async move {
                let upload = Answer::Upload {
                    message,
                    session,
                    key,
                    step,
                };
                let _closed = events.send(Event::Files(upload)).await;
            }
        };
        let step = match client {
            Ok(client) => match upload(&client, attach, &tell).await {
                Ok(attachment) => Step::Done(attachment),
                Err((problem, again)) => Step::Failed { problem, again },
            },
            Err(why) => Step::Failed {
                problem: Problem::Node(why.to_string()),
                again: Again::Restart,
            },
        };
        tell(step).await;
    }));
}

/// A prompt's content: its text (when there is any), then each file, as
/// the CLI's `vessel send --attach` orders them.
pub(crate) fn content(text: String, attachments: Vec<AttachmentRef>) -> Vec<ContentBlock> {
    let text = (!text.is_empty()).then_some(ContentBlock::Text { text });
    text.into_iter()
        .chain(
            attachments
                .into_iter()
                .map(|attachment| ContentBlock::Attachment { attachment }),
        )
        .collect()
}

/// What failed, and how it may be tried again.
type Failure = (Problem, Again);

async fn upload<F, T>(
    client: &SelectedConnection,
    attach: Attach,
    tell: &F,
) -> Result<AttachmentRef, Failure>
where
    F: Fn(Step) -> T,
    T: Future<Output = ()>,
{
    let source = Source::of(&attach.origin);
    if attach.resume.is_none() {
        let head = source.head().await?;
        if !looks_like(&attach.mime, &head) {
            return Err((Problem::NotItsType, Again::Never));
        }
    }
    let mut begun = if let Some(begun) = attach.resume {
        begun
    } else {
        let upload = client
            .call(vessel_proto::conversation::BeginUpload {
                session: attach.session,
                name: attach.name.clone(),
                mime: attach.mime.clone(),
                size: attach.size,
            })
            .await
            .map_err(|error| node(&error, None))?;
        Begun {
            upload: upload.upload,
            chunk: upload.chunk,
            received: 0,
        }
    };
    let piece = usize::try_from(begun.chunk)
        .unwrap_or(BlobChunk::MAX_LEN)
        .clamp(1, BlobChunk::MAX_LEN);
    while begun.received < attach.size {
        let left = attach.size.saturating_sub(begun.received);
        let length = usize::try_from(left).unwrap_or(usize::MAX).min(piece);
        let data = source
            .piece(begun.received, length)
            .await
            .map_err(|problem| {
                // A file that changed cannot go on where it was: attached again.
                let again = if problem == Problem::Changed {
                    Again::Never
                } else {
                    Again::Resume(begun)
                };
                (problem, again)
            })?;
        let written = client
            .call(vessel_proto::conversation::WriteUpload {
                upload: begun.upload,
                offset: begun.received,
                data,
            })
            .await
            .map_err(|error| node(&error, Some(begun)))?;
        if written.received <= begun.received {
            // A node that takes nothing would hold the task here for ever.
            let why = format!("the node took no more after {} bytes", begun.received);
            return Err((Problem::Node(why), Again::Resume(begun)));
        }
        begun.received = written.received;
        tell(Step::Progress(begun.received)).await;
    }
    client
        .call(vessel_proto::conversation::FinishUpload {
            upload: begun.upload,
        })
        .await
        .map_err(|error| node(&error, Some(begun)))
}

/// The node's refusal or the connection's failure, and whether to go on:
/// an upload the node no longer knows is begun again, a file the node
/// judged wrong is not tried again, anything else goes on from what the
/// node has.
fn node(error: &RequestFailure, begun: Option<Begun>) -> Failure {
    let projected = super::selected::problem(error);
    let RequestFailure::Call(error) = error else {
        return (Problem::Node(projected.to_string()), Again::Never);
    };
    let again = match error {
        CallError::Remote(wire) => match wire.code {
            ErrorCode::NotFound => Again::Restart,
            ErrorCode::InvalidRequest | ErrorCode::Unsupported | ErrorCode::Conflict => {
                Again::Never
            }
            ErrorCode::Unauthorized
            | ErrorCode::Forbidden
            | ErrorCode::ProtocolMismatch
            | ErrorCode::HarnessNotInstalled
            | ErrorCode::SpawnFailed
            | ErrorCode::Unavailable
            | ErrorCode::Internal => begun.map_or(Again::Restart, Again::Resume),
        },
        CallError::Disconnected
        | CallError::UnexpectedReply(_)
        | CallError::RequestIdsExhausted
        | CallError::NotInProtocol { .. }
        | CallError::UnassignedProtocol { .. } => begun.map_or(Again::Restart, Again::Resume),
    };
    (Problem::Node(projected.to_string()), again)
}

/// Where the bytes are read from: a file is opened for each piece on the
/// blocking pool (as the link reads its other files), so no handle is
/// held between pieces and the task never blocks the runtime.
enum Source<'a> {
    File(PathBuf),
    Pasted(&'a [u8]),
}

impl<'a> Source<'a> {
    fn of(origin: &'a Origin) -> Self {
        match origin {
            Origin::File(path) => Self::File(path.clone()),
            Origin::Pasted(bytes) => Self::Pasted(bytes.as_ref()),
            Origin::Stored(local) => Self::File(local.path.clone()),
        }
    }

    /// The first bytes, for the type's check (fewer for a shorter file).
    async fn head(&self) -> Result<Vec<u8>, Failure> {
        match self {
            Self::File(path) => {
                let path = path.clone();
                blocking(move || {
                    let mut head = Vec::with_capacity(HEAD);
                    let _read = std::fs::File::open(&path)?
                        .take(u64::try_from(HEAD).unwrap_or(u64::MAX))
                        .read_to_end(&mut head)?;
                    Ok(head)
                })
                .await
                .map_err(|problem| (problem, Again::Restart))
            }
            Self::Pasted(bytes) => Ok(bytes.iter().take(HEAD).copied().collect()),
        }
    }

    /// `length` bytes from `offset`, one buffer per piece, handed over to
    /// be sent.
    async fn piece(&self, offset: u64, length: usize) -> Result<Bytes, Problem> {
        match self {
            Self::File(path) => {
                let path = path.clone();
                blocking(move || read_piece(&path, offset, length))
                    .await
                    .map(Bytes::from)
            }
            Self::Pasted(bytes) => {
                let start = usize::try_from(offset).unwrap_or(usize::MAX);
                bytes
                    .get(start..start.saturating_add(length))
                    .map(Bytes::copy_from_slice)
                    .ok_or(Problem::Changed)
            }
        }
    }
}

fn read_piece(path: &Path, offset: u64, length: usize) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let _at = file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0_u8; length];
    file.read_exact(&mut buffer)?;
    Ok(buffer)
}

/// `read` on the blocking pool; a file that ended early is one that
/// changed while it was sent.
async fn blocking<T, F>(read: F) -> Result<T, Problem>
where
    T: Send + 'static,
    F: FnOnce() -> std::io::Result<T> + Send + 'static,
{
    match tokio::task::spawn_blocking(read).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
            Err(Problem::Changed)
        }
        Ok(Err(error)) => Err(Problem::Unreadable(error.to_string())),
        Err(error) => Err(Problem::Unreadable(error.to_string())),
    }
}

#[cfg(test)]
mod tests;
