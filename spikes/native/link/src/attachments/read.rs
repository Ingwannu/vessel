//! Read only displayed images; reject malformed totals before retaining bytes.
//!
//! The client's generic transfer sink requires a manifest hash and writes
//! on a blocking thread. Attachment references have no manifest hash and
//! these bounded bytes belong in memory, so this collector validates the
//! attachment stream directly rather than pretending it is a transfer.

use super::{Answer, ReadProblem};
use crate::connections::SelectedConnection;
use crate::{Event, LinkError};
use std::sync::Arc;
use tokio::sync::mpsc;
use vessel_app_view::composer::sent::Sent;
use vessel_proto::{AttachmentLimits, BlobChunk};

/// Keep reads outside the command loop so other UI operations stay live.
pub(crate) fn spawn(
    client: Result<SelectedConnection, LinkError>,
    sent: Sent,
    events: mpsc::Sender<Event>,
) {
    drop(tokio::spawn(async move {
        let result = match client {
            Ok(client) => read(&client, &sent).await,
            Err(why) => Err(why),
        };
        let bytes = match result {
            Ok(bytes) => Some(bytes),
            Err(why) => {
                tracing::info!(%why, attachment = %sent.attachment, "a sent image was not read back");
                None
            }
        };
        let _closed = events
            .send(Event::Files(Answer::Read { sent, bytes }))
            .await;
    }));
}

/// A peer that stays connected must not hold one of the two preview slots forever.
const READ_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

async fn read(client: &SelectedConnection, sent: &Sent) -> Result<Arc<[u8]>, LinkError> {
    tokio::time::timeout(READ_DEADLINE, download(client, sent))
        .await
        .map_err(|_elapsed| LinkError::AttachmentRead(ReadProblem::TimedOut))?
}

async fn download(client: &SelectedConnection, sent: &Sent) -> Result<Arc<[u8]>, LinkError> {
    let mut image = Collector::default();
    loop {
        let chunk = client
            .call(vessel_proto::conversation::ReadAttachment {
                session: sent.session,
                attachment: sent.attachment,
                offset: image.offset(),
            })
            .await
            .map_err(|error| {
                LinkError::AttachmentRead(ReadProblem::Request(super::selected::problem(&error)))
            })?;
        if image.take(&chunk).map_err(LinkError::AttachmentRead)? == Next::Done {
            return Ok(Arc::from(image.bytes));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    More,
    Done,
}

/// An immutable node file has one size throughout a read. Verify it before
/// allocation or append, including when a malicious peer says it is done.
#[derive(Default)]
struct Collector {
    bytes: Vec<u8>,
    total: Option<u64>,
}

impl Collector {
    fn offset(&self) -> u64 {
        u64::try_from(self.bytes.len()).unwrap_or(u64::MAX)
    }

    fn take(&mut self, chunk: &BlobChunk) -> Result<Next, ReadProblem> {
        if chunk.total == 0 {
            return Err(ReadProblem::Empty);
        }
        if chunk.total > AttachmentLimits::DEFAULT.image {
            return Err(ReadProblem::TooLarge);
        }
        if self.total.is_some_and(|total| total != chunk.total) {
            return Err(ReadProblem::Changed);
        }
        let len = u64::try_from(chunk.data.len()).unwrap_or(u64::MAX);
        let next = self.offset().saturating_add(len);
        if next > chunk.total {
            return Err(ReadProblem::Overrun);
        }
        if len == 0 && next < chunk.total {
            return Err(ReadProblem::Stopped);
        }
        self.total = Some(chunk.total);
        self.bytes.extend_from_slice(&chunk.data);
        Ok(if next == chunk.total {
            Next::Done
        } else {
            Next::More
        })
    }
}

#[cfg(test)]
pub(in crate::attachments) mod tests;
