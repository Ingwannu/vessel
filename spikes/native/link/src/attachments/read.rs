//! A sent image read back from its node (track P's row c, its third step):
//! a mebibyte at a time (`attachment.read`), from the start to the size the
//! node gives, beside the link's other commands. Which images are read,
//! and when, is the view's (`composer::sent`).

use std::sync::Arc;

use tokio::sync::mpsc;
use vessel_app_view::composer::sent::Sent;
use vessel_client::NodeClient;
use vessel_proto::{AttachmentLimits, BlobChunk};

use super::Answer;
use crate::Event;

/// Read `sent` back beside the link's other commands, answering
/// `Answer::Read`; `client` is the machine's connection, or why there is
/// none.
pub(crate) fn spawn(client: Result<NodeClient, String>, sent: Sent, events: mpsc::Sender<Event>) {
    drop(tokio::spawn(async move {
        let read = match client {
            Ok(client) => read(&client, &sent).await,
            Err(why) => Err(why),
        };
        let bytes = match read {
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

/// The image's bytes, piece after piece until the node's size.
async fn read(client: &NodeClient, sent: &Sent) -> Result<Arc<[u8]>, String> {
    let most = AttachmentLimits::DEFAULT.image;
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let offset = u64::try_from(bytes.len()).map_err(|error| error.to_string())?;
        let chunk = client
            .read_attachment(sent.session, sent.attachment, offset)
            .await
            .map_err(|error| error.to_string())?;
        if take(&mut bytes, &chunk, most)? == Next::Done {
            return Ok(Arc::from(bytes));
        }
    }
}

/// What a piece leaves to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    /// Ask for the piece after it.
    More,
    /// The image is whole.
    Done,
}

/// `chunk` taken into `bytes`: done once they hold the node's size. An
/// image larger than `most` is not read (a node never took one), and a
/// piece that brings nothing before the end stops the read (the node
/// would be asked again for ever).
fn take(bytes: &mut Vec<u8>, chunk: &BlobChunk, most: u64) -> Result<Next, String> {
    if chunk.total > most {
        return Err(format!("{} bytes, more than an image may be", chunk.total));
    }
    if bytes.is_empty() {
        bytes.reserve(usize::try_from(chunk.total).unwrap_or(0));
    }
    bytes.extend_from_slice(&chunk.data);
    let held = u64::try_from(bytes.len()).map_err(|error| error.to_string())?;
    if held >= chunk.total {
        return Ok(Next::Done);
    }
    if chunk.data.is_empty() {
        return Err(format!(
            "the node stopped at {held} of {} bytes",
            chunk.total
        ));
    }
    Ok(Next::More)
}

#[cfg(test)]
mod tests;
