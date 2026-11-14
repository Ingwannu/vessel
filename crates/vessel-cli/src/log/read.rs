//! Snapshot history reuses the existing reader, with a requested tail window.
//! No capacity is reserved from an arbitrary count and no wire call is added.

use vessel_client::NodeClient;
use vessel_proto::{EventSeq, SessionId};

use super::tail::{Tail, Window};
use super::{Run, Stream, pruned, render};
use crate::error::CliError;
use crate::io;
use crate::view::Conversation;

/// The existing page budget bounds every immediate render batch.
const PAGE: u32 = 500;

/// Print history from its sequence floor, optionally holding only the requested tail.
pub(crate) async fn print(
    client: &NodeClient,
    session: SessionId,
    mut next: EventSeq,
    tail: Option<Tail>,
) -> Result<(), CliError> {
    if tail == Some(Tail::ZERO) {
        return Ok(());
    }
    let mut conversation = Conversation::default();
    let mut held = tail.map(Window::new);
    loop {
        let page = client.read(session, next, PAGE).await?;
        match &mut held {
            Some(window) => {
                if let Some(notice) = pruned(next, &page.records) {
                    write(vec![notice]).await?;
                }
                for record in page.records {
                    window.push(record);
                }
            }
            None => write(render(&mut conversation, next, &page.records)).await?,
        }
        next = page.next;
        if page.at_end {
            break;
        }
    }
    if let Some(window) = held {
        let mut records = window.into_records();
        if let Some(first) = records.front() {
            // Pruning was checked on raw pages; omitted history belongs to the tail.
            write(render(
                &mut conversation,
                first.seq,
                records.make_contiguous(),
            ))
            .await?;
        }
    }
    Ok(())
}

async fn write(runs: Vec<Run>) -> Result<(), CliError> {
    for run in runs {
        match run.stream {
            Stream::Out => io::out(&run.text).await?,
            Stream::Note => io::note_bytes(&run.text).await?,
        }
    }
    Ok(())
}
