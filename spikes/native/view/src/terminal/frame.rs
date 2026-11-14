//! What a terminal session's frame says around the emulator (the page's
//! `features/session/terminal-view.tsx`): the size it is drawn at, a
//! routed session's model calls, whether it is live; and when a view may
//! resize the session.

use vessel_proto::{EventRecord, EventSeq, SessionEvent};

use crate::timeline::ModelUsage;

/// Whether the terminal is live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// Following the session.
    Live,
    /// The stream is reconnecting or failed: what is drawn may be stale.
    Frozen,
    /// The session ended.
    Ended,
}

/// When a view may send its size: only one that has focus and has typed
/// (Paseo's rule), so a view that only watches never resizes the harness
/// another person is typing into.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResizeRule {
    typed: bool,
}

impl ResizeRule {
    /// The person typed in this view. Says whether this is the first
    /// time: the view's size is then sent before the keys.
    pub const fn typed(&mut self) -> bool {
        let first = !self.typed;
        self.typed = true;
        first
    }

    /// Whether a view focused or not (`focused`) may send its size now.
    #[must_use]
    pub const fn may_resize(self, focused: bool) -> bool {
        self.typed && focused
    }
}

/// A terminal session's log as its view reads it: the bytes to draw, and
/// its model calls summed.
#[derive(Debug)]
pub struct TerminalLog {
    next: EventSeq,
    usage: Option<ModelUsage>,
}

impl Default for TerminalLog {
    fn default() -> Self {
        Self {
            next: EventSeq::ZERO,
            usage: None,
        }
    }
}

impl TerminalLog {
    /// `records` read: the bytes of their terminal output, in order, for
    /// the emulator. Records already read (a stream that starts again)
    /// are skipped, so no call is counted twice.
    pub fn apply<'a>(&mut self, records: &'a [EventRecord]) -> Vec<&'a [u8]> {
        let mut bytes = Vec::new();
        for record in records {
            if record.seq < self.next {
                continue;
            }
            let Some(next) = record.seq.next() else {
                break;
            };
            self.next = next;
            match &record.event {
                SessionEvent::Terminal(output) => bytes.push(output.data.bytes().as_ref()),
                SessionEvent::ModelCall(call) => {
                    self.usage.get_or_insert_with(ModelUsage::default).add(call);
                }
                SessionEvent::Status { .. }
                | SessionEvent::Agent { .. }
                | SessionEvent::ProfileChanged { .. }
                | SessionEvent::OptionsChanged { .. }
                | SessionEvent::Renamed { .. }
                | SessionEvent::QueueChanged { .. }
                | SessionEvent::CommandsChanged { .. }
                | SessionEvent::UsageChanged { .. }
                | SessionEvent::Parked { .. }
                | SessionEvent::Woken
                | SessionEvent::Forked { .. }
                | SessionEvent::Rewound { .. } => {}
            }
        }
        bytes
    }

    /// The model calls so far; `None` before the first.
    #[must_use]
    pub const fn usage(&self) -> Option<&ModelUsage> {
        self.usage.as_ref()
    }

    /// The next record expected: where a resumed stream starts.
    #[must_use]
    pub const fn next(&self) -> EventSeq {
        self.next
    }
}
