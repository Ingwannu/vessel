//! `vessel log`: a session's log, printed in record order.
//!
//! [Decision Log]
//! - Purpose: the log must read in sequence order whether stdout and
//!   stderr go to one terminal, are merged (`2>&1`), or are split
//!   (`> session.log`).
//! - Constraint: the session's own output (terminal bytes, the
//!   conversation) goes to stdout and Vessel's status lines to stderr, so
//!   `vessel log ID > file` captures exactly what the harness printed and
//!   `cat file` replays its screen. Two descriptors have no common order
//!   except the order of the writes.
//! - Alternatives: everything on stdout (simple, but status lines would end
//!   up inside the captured terminal bytes, in the middle of a replayed
//!   screen); one write per stream per page, as before (statuses of a page
//!   then came out after all of its output).
//! - Choice: render each page into runs of consecutive records that go to
//!   the same stream, and write the runs in order, each flushed before the
//!   next starts (`io::out` and `io::note` flush). Merged output is then in
//!   record order, and split output is as clean as before.
//! - Cost: one write per change of stream instead of two per page; status
//!   records are few, so this is a handful of extra writes per session.

use std::borrow::Cow;

use vessel_proto::{EventRecord, EventSeq, SessionEvent};

use crate::view::Conversation;

mod read;
pub(crate) mod select;
pub(crate) mod tail;

pub(crate) use read::print;

/// Rendering survives page and history/live boundaries, with gaps separating messages.
#[derive(Default)]
struct Timeline {
    conversation: Conversation,
    previous: Option<EventSeq>,
}

/// Which stream a run of text goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stream {
    /// The session's output: stdout.
    Out,
    /// Vessel's notes about the session: stderr.
    Note,
}

/// Consecutive text for one stream.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    stream: Stream,
    text: Vec<u8>,
}

/// A page as runs in record order. `expected` is the first record asked
/// for; a later first record means the ones before it were pruned.
#[must_use]
fn render(timeline: &mut Timeline, expected: EventSeq, records: &[EventRecord]) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut push = |stream: Stream, text: &[u8]| {
        if text.is_empty() {
            return;
        }
        match runs.last_mut() {
            Some(run) if run.stream == stream => run.text.extend_from_slice(text),
            Some(_) | None => runs.push(Run {
                stream,
                text: text.to_vec(),
            }),
        }
    };
    if let Some(notice) = pruned(expected, records) {
        push(notice.stream, &notice.text);
    }
    for record in records {
        if timeline
            .previous
            .is_some_and(|previous| previous.next() != Some(record.seq))
        {
            push(Stream::Out, timeline.conversation.finish_line().as_bytes());
        }
        timeline.previous = Some(record.seq);
        let (stream, text) = timeline.show(record);
        push(stream, &text);
    }
    runs
}

impl Timeline {
    #[must_use]
    fn show<'a>(&mut self, record: &'a EventRecord) -> (Stream, Cow<'a, [u8]>) {
        match &record.event {
            SessionEvent::Terminal(output) => (Stream::Out, Cow::Borrowed(output.data.bytes())),
            SessionEvent::Status { .. }
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. }
            | SessionEvent::Reloaded { .. } => (
                Stream::Note,
                Cow::Owned(self.conversation.show(record).into_bytes()),
            ),
            SessionEvent::Agent { .. } => (
                Stream::Out,
                Cow::Owned(self.conversation.show(record).into_bytes()),
            ),
        }
    }
}

/// Only a raw page can prove pruning: selection gaps must never become this notice.
#[must_use]
fn pruned(expected: EventSeq, records: &[EventRecord]) -> Option<Run> {
    let first = records.first()?;
    (first.seq > expected).then(|| Run {
        stream: Stream::Note,
        text: format!("[vessel] records {expected}..{} were pruned\n", first.seq).into_bytes(),
    })
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use chrono::DateTime;
    use vessel_proto::{
        AgentEvent, ContentBlock, HarnessSessionId, SessionStatus, TerminalBytes, TerminalOutput,
    };

    use super::*;

    fn record(seq: u64, event: SessionEvent) -> EventRecord {
        EventRecord {
            seq: EventSeq::new(seq),
            at: DateTime::UNIX_EPOCH,
            event,
        }
    }

    fn status(seq: u64, status: SessionStatus) -> EventRecord {
        record(seq, SessionEvent::Status { status })
    }

    fn agent(seq: u64, event: AgentEvent) -> EventRecord {
        record(seq, SessionEvent::Agent { event })
    }

    fn terminal(seq: u64, text: &'static str) -> EventRecord {
        record(
            seq,
            SessionEvent::Terminal(TerminalOutput {
                data: TerminalBytes::new(Bytes::from_static(text.as_bytes())),
            }),
        )
    }

    fn runs(records: &[EventRecord], expected: u64) -> Vec<(Stream, String)> {
        render(&mut Timeline::default(), EventSeq::new(expected), records)
            .into_iter()
            .map(|run| (run.stream, String::from_utf8(run.text).unwrap()))
            .collect()
    }

    #[test]
    fn statuses_and_output_keep_record_order_across_streams() {
        let records = [
            status(0, SessionStatus::Starting),
            agent(
                1,
                AgentEvent::Ready {
                    harness_session: HarnessSessionId::new("s-1"),
                    abilities: None,
                },
            ),
            status(2, SessionStatus::Running),
            agent(
                3,
                AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: "hello".to_owned(),
                    },
                },
            ),
            status(4, SessionStatus::Interrupted),
        ];
        assert_eq!(
            runs(&records, 0),
            [
                (Stream::Note, "[session starting]\n".to_owned()),
                (Stream::Out, "[ready: harness session s-1]\n".to_owned()),
                (Stream::Note, "[session running]\n".to_owned()),
                (Stream::Out, "hello".to_owned()),
                (Stream::Note, "\n[session interrupted]\n".to_owned()),
            ]
        );
    }

    #[test]
    fn a_profile_change_is_a_note() {
        let records = [
            record(
                0,
                SessionEvent::ProfileChanged {
                    from: None,
                    to: Some("alt".parse().unwrap()),
                },
            ),
            status(1, SessionStatus::Running),
        ];
        assert_eq!(
            runs(&records, 0),
            [(
                Stream::Note,
                "[profile default -> alt]\n[session running]\n".to_owned()
            )]
        );
    }

    #[test]
    fn consecutive_records_of_one_stream_are_one_write() {
        let records = [
            terminal(5, "a"),
            terminal(6, "b"),
            status(7, SessionStatus::Exited { code: Some(0) }),
        ];
        assert_eq!(
            runs(&records, 3),
            [
                (
                    Stream::Note,
                    "[vessel] records 3..5 were pruned\n".to_owned()
                ),
                (Stream::Out, "ab".to_owned()),
                (Stream::Note, "[session exited (0)]\n".to_owned()),
            ]
        );
    }
    #[test]
    fn selection_gaps_separate_messages_without_a_false_retention_notice() {
        let chunk = |seq, text: &str| {
            agent(
                seq,
                AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: text.to_owned(),
                    },
                },
            )
        };
        let mut timeline = Timeline::default();
        let first = render(&mut timeline, EventSeq::new(4), &[chunk(4, "one")]);
        let second = render(
            &mut timeline,
            EventSeq::new(9),
            &[chunk(9, "two"), chunk(10, "!")],
        );
        assert_eq!(
            first,
            [Run {
                stream: Stream::Out,
                text: b"one".to_vec()
            }]
        );
        assert_eq!(
            second,
            [Run {
                stream: Stream::Out,
                text: b"\ntwo!".to_vec()
            }]
        );
    }
    #[test]
    fn a_tail_counts_raw_chunks_and_can_begin_inside_a_rendered_message() {
        let mut window = tail::Window::new("1".parse().unwrap());
        for (seq, text) in [(0, "hel"), (1, "lo")] {
            window.push(agent(
                seq,
                AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: text.to_owned(),
                    },
                },
            ));
        }
        let mut records = window.into_records();
        let expected = records.front().unwrap().seq;
        assert_eq!(
            render(
                &mut Timeline::default(),
                expected,
                records.make_contiguous()
            ),
            [Run {
                stream: Stream::Out,
                text: b"lo".to_vec()
            }]
        );
    }
}
