//! Snapshot selection never changes the raw page cursor.
//!
//! The existing reader owns catch-up and retention. Only tail requests hold
//! records, evicting the oldest match as soon as their requested count is full.
//! No capacity is reserved from an untrusted count, and no wire call is added.

use vessel_client::NodeClient;
use vessel_proto::{EventRecord, EventSeq, SessionId};

use super::select::Selection;
use super::tail::{Tail, Window};
use super::{Run, Stream, Timeline, pruned, render};
use crate::args::Log;
use crate::error::CliError;
use crate::io;

/// The existing page budget also bounds each untailed selection batch.
const PAGE: u32 = 500;

/// Print a snapshot using predicates before the requested tail window.
pub(crate) async fn print(
    client: &NodeClient,
    session: SessionId,
    options: &Log,
) -> Result<(), CliError> {
    if options.tail == Some(Tail::ZERO) {
        return Ok(());
    }
    history(
        client,
        session,
        options.from,
        options.selection(),
        options.tail,
        &mut Timeline::default(),
    )
    .await
}

async fn history(
    client: &NodeClient,
    session: SessionId,
    mut next: EventSeq,
    selection: Selection,
    tail: Option<Tail>,
    timeline: &mut Timeline,
) -> Result<(), CliError> {
    let mut held = tail.map(Window::new);
    loop {
        let page = client.read(session, next, PAGE).await?;
        if let Some(notice) = pruned(next, &page.records) {
            write(vec![notice]).await?;
        }
        let selected: Vec<_> = page
            .records
            .into_iter()
            .filter(|record| selection.keeps(record))
            .collect();
        match &mut held {
            Some(window) => {
                for record in selected {
                    window.push(record);
                }
            }
            None => write_selected(timeline, &selected).await?,
        }
        next = page.next;
        if page.at_end {
            break;
        }
    }
    if let Some(window) = held {
        let mut records = window.into_records();
        write_selected(timeline, records.make_contiguous()).await?;
    }
    Ok(())
}

async fn write_selected(timeline: &mut Timeline, records: &[EventRecord]) -> Result<(), CliError> {
    if let Some(first) = records.first() {
        // Pruning was checked on the raw page; a later first match is selection.
        write(render(timeline, first.seq, records)).await?;
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
