//! Page history before following its cursor; selection never changes that cursor.
//!
//! The existing reader owns catch-up and retention. Only tail requests hold
//! records, evicting the oldest match as soon as their requested count is full.
//! No capacity is reserved from an untrusted count, and no wire call is added.

use vessel_client::NodeClient;
use vessel_proto::{EventRecord, EventSeq, SessionId, SessionInfo};

use super::select::Selection;
use super::tail::{Tail, Window};
use super::{Run, Stream, Timeline, pruned, render};
use crate::args::{Log, LogMode};
use crate::error::CliError;
use crate::io;

/// The existing page budget also bounds each untailed selection batch.
const PAGE: u32 = 500;

/// Print selected history, then optionally reuse the gap-free session stream.
pub(crate) async fn print(
    client: &NodeClient,
    info: &SessionInfo,
    options: &Log,
) -> Result<(), CliError> {
    let mut timeline = Timeline::default();
    let selection = options.selection();
    let tail = options.history_tail();
    let next = if tail == Some(Tail::ZERO) {
        // The info snapshot defines "no history"; subsequent records are live.
        info.next_seq.max(options.from)
    } else {
        history(
            client,
            info.id,
            options.from,
            selection,
            tail,
            &mut timeline,
        )
        .await?
    };
    match options.mode {
        LogMode::Once => Ok(()),
        LogMode::Follow => follow(client, info.id, next, selection, &mut timeline).await,
    }
}

async fn history(
    client: &NodeClient,
    session: SessionId,
    mut next: EventSeq,
    selection: Selection,
    tail: Option<Tail>,
    timeline: &mut Timeline,
) -> Result<EventSeq, CliError> {
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
    Ok(next)
}

async fn follow(
    client: &NodeClient,
    session: SessionId,
    next: EventSeq,
    selection: Selection,
    timeline: &mut Timeline,
) -> Result<(), CliError> {
    let mut stream = client.follow(session, next).await?;
    // Keep the receiver alive during output awaits so a signal remains pending.
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);
    loop {
        let item = tokio::select! {
            biased;
            interrupted = &mut interrupt => {
                interrupted?;
                return finish(timeline).await;
            }
            item = stream.next() => item,
        };
        match item {
            None => return finish(timeline).await,
            Some(Ok(record)) => {
                if selection.keeps(&record) {
                    write_selected(timeline, std::slice::from_ref(&record)).await?;
                }
            }
            Some(Err(error)) => {
                finish(timeline).await?;
                if error.is_fatal() {
                    return Err(error.into());
                }
                io::note(&format!("[vessel] {error}\n")).await?;
            }
        }
    }
}

/// Complete an open message before EOF, interruption or a transport notice.
async fn finish(timeline: &mut Timeline) -> Result<(), CliError> {
    Ok(io::out(timeline.conversation.finish_line().as_bytes()).await?)
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
