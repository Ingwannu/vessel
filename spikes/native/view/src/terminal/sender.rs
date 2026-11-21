//! A terminal view's keys and sizes on their way to the session, in order
//! (the page's `data/terminal-sender.ts`, itself the CLI's attach sender).
//!
//! [Decision Log]
//! - Purpose: what a person types reaches the harness in the order it was
//!   typed, a paste and the Enter after it included.
//! - Constraint: the node runs a connection's calls concurrently, so two
//!   sends in flight at once can reach the terminal in either order.
//! - Choice: one send in flight per view; keys typed meanwhile are joined
//!   into one send, and a size queued just before another is replaced,
//!   keys and sizes keeping their order. A pure state machine here: the
//!   screen takes the next send from it and tells it when that send has
//!   answered, so the order holds whatever runs the call.
//! - Bound: at most `MAX_PENDING` characters of typed keys wait; past
//!   that, keys are dropped and the screen is told once per overflow (an
//!   unbounded queue behind a harness that stopped reading would grow
//!   without end).
//! - Pastes (P1 review note 2): a paste is queued whole or not at all, so
//!   no part of one is ever lost; it waits beside the typed keys, keys
//!   typed after it go after it, and it goes out in sends the node takes.
//!   Paseo bounds neither; pastes waiting are bounded by
//!   `MAX_PASTE_PENDING` bytes, past which a paste is refused whole (held
//!   key repeat on a huge clipboard would otherwise grow without end).
//! - Send size: one send carries at most the node's
//!   `SendTerminal::MAX_BYTES` (64 KiB), cut at a character boundary, so
//!   a long paste or a long burst of keys is several sends in order; the
//!   node refuses a larger one whole. A queued text keeps an offset past
//!   the parts sent, so each send copies only its own part (P3 app review
//!   note 2: splitting the rest off moved it again on every send, about
//!   8 GiB for a 32 MiB paste, on the window's thread).
//! - Failures: a send that fails is not repeated (it may have been
//!   applied); later items still go, in order. The rest of a paste whose
//!   part failed is dropped, so a paste never lands with a hole in it
//!   (P4w node review note 1): what got through is its start, and keys
//!   typed after it still go.
//! - Sizes (ADR 0116): each names a claim or an update. A size replacing a
//!   queued claim keeps the claim, so a focus followed at once by a fit
//!   still takes the terminal.

use std::collections::VecDeque;

use vessel_proto::ResizeIntent;
use vessel_proto::terminals::SendTerminal;

/// Most characters of typed keys waiting while a send is in flight.
pub const MAX_PENDING: usize = 1024 * 1024;

/// Most bytes of pastes waiting; a paste that would pass it is refused
/// whole.
pub const MAX_PASTE_PENDING: usize = 32 * 1024 * 1024;

/// Most bytes one send carries (the node's own bound).
pub const MAX_SEND_BYTES: usize = SendTerminal::MAX_BYTES;

/// One send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outgoing {
    /// Keys, as typed (UTF-8).
    Keys(String),
    /// The view's size.
    Resize {
        /// Columns.
        cols: u16,
        /// Rows.
        rows: u16,
        /// Whether it takes the terminal's size or is a later fit of its
        /// owner (ADR 0116; a node below 37 is sent neither).
        intent: ResizeIntent,
    },
}

/// What happened to keys handed to the sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taken {
    /// Queued (or nothing to queue).
    Queued,
    /// Dropped, and this is the first drop of an overflow: say so once.
    DroppedFirst,
    /// Dropped again in the same overflow.
    Dropped,
}

/// A queued item, by where its text came from, so each kind keeps its
/// own bound.
#[derive(Debug)]
enum Item {
    Typed(Queued),
    Pasted(Queued),
    Resize {
        cols: u16,
        rows: u16,
        intent: ResizeIntent,
    },
}

/// What send is in flight.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum InFlight {
    /// None: the next may go.
    #[default]
    Nothing,
    /// A send whose failure drops nothing more.
    Send,
    /// A part of a paste with more to go: its failure drops the rest.
    PastePart,
}

/// The sender's queue.
#[derive(Debug, Default)]
pub struct TerminalSender {
    items: VecDeque<Item>,
    /// Characters of typed keys queued.
    chars: usize,
    /// Bytes of pastes queued.
    pasted: usize,
    in_flight: InFlight,
    closed: bool,
    overflowing: bool,
}

impl TerminalSender {
    /// Queue keys, joined to keys typed just before.
    pub fn keys(&mut self, text: &str) -> Taken {
        if self.closed || text.is_empty() {
            return Taken::Queued;
        }
        let count = text.chars().count();
        if self.chars.saturating_add(count) > MAX_PENDING {
            return self.overflow();
        }
        self.overflowing = false;
        self.chars = self.chars.saturating_add(count);
        if let Some(Item::Typed(last)) = self.items.back_mut() {
            last.text.push_str(text);
        } else {
            self.items
                .push_back(Item::Typed(Queued::new(text.to_owned())));
        }
        Taken::Queued
    }

    /// Queue a paste, already encoded (`encode_paste`: bracketed once
    /// around the whole of it when the program asked), whole or not at all.
    pub fn paste(&mut self, encoded: String) -> Taken {
        if self.closed || encoded.is_empty() {
            return Taken::Queued;
        }
        if self.pasted.saturating_add(encoded.len()) > MAX_PASTE_PENDING {
            return self.overflow();
        }
        self.overflowing = false;
        self.pasted = self.pasted.saturating_add(encoded.len());
        self.items.push_back(Item::Pasted(Queued::new(encoded)));
        Taken::Queued
    }

    fn overflow(&mut self) -> Taken {
        if self.overflowing {
            return Taken::Dropped;
        }
        self.overflowing = true;
        Taken::DroppedFirst
    }

    /// Queue a size; a size queued just before is replaced. A claim it
    /// replaces stays a claim: the later size is what the claim takes.
    pub fn resize(&mut self, cols: u16, rows: u16, intent: ResizeIntent) {
        if self.closed {
            return;
        }
        if let Some(Item::Resize {
            cols: last_cols,
            rows: last_rows,
            intent: last_intent,
        }) = self.items.back_mut()
        {
            *last_cols = cols;
            *last_rows = rows;
            if intent == ResizeIntent::Claim {
                *last_intent = ResizeIntent::Claim;
            }
        } else {
            self.items.push_back(Item::Resize { cols, rows, intent });
        }
    }

    /// The next send, when none is in flight: it is then in flight until
    /// `answered`. Keys longer than one send's bound go a part at a time.
    pub fn take(&mut self) -> Option<Outgoing> {
        if self.in_flight != InFlight::Nothing || self.closed {
            return None;
        }
        let outgoing = match self.items.front_mut()? {
            Item::Typed(queued) => {
                let part = queued.next_part();
                self.chars = self.chars.saturating_sub(part.chars().count());
                Outgoing::Keys(part)
            }
            Item::Pasted(queued) => {
                let part = queued.next_part();
                self.pasted = self.pasted.saturating_sub(part.len());
                Outgoing::Keys(part)
            }
            Item::Resize { cols, rows, intent } => Outgoing::Resize {
                cols: *cols,
                rows: *rows,
                intent: *intent,
            },
        };
        let spent = match self.items.front() {
            Some(Item::Typed(queued) | Item::Pasted(queued)) => queued.is_spent(),
            Some(Item::Resize { .. }) | None => true,
        };
        let part = !spent && matches!(self.items.front(), Some(Item::Pasted(_)));
        if spent {
            let _sent = self.items.pop_front();
        }
        self.in_flight = if part {
            InFlight::PastePart
        } else {
            InFlight::Send
        };
        Some(outgoing)
    }

    /// The send in flight was taken. The next one may go.
    pub const fn answered(&mut self) {
        self.in_flight = InFlight::Nothing;
    }

    /// The send in flight was refused or lost: it is not repeated, and the
    /// rest of a paste it was a part of is dropped. The next one may go.
    pub fn failed(&mut self) {
        if std::mem::take(&mut self.in_flight) == InFlight::PastePart
            && let Some(Item::Pasted(queued)) = self.items.front()
        {
            let rest = queued.text.len().saturating_sub(queued.at);
            self.pasted = self.pasted.saturating_sub(rest);
            let _dropped = self.items.pop_front();
        }
    }

    /// Drop what has not been sent; send nothing more.
    pub fn close(&mut self) {
        self.closed = true;
        self.items.clear();
        self.chars = 0;
        self.pasted = 0;
    }

    /// Whether a send is in flight.
    #[must_use]
    pub const fn busy(&self) -> bool {
        !matches!(self.in_flight, InFlight::Nothing)
    }
}

/// Text waiting to go out a part at a time; `at` is past the parts sent.
#[derive(Debug)]
struct Queued {
    text: String,
    at: usize,
}

impl Queued {
    const fn new(text: String) -> Self {
        Self { text, at: 0 }
    }

    /// The next part, at most one send's bound, cut at a character
    /// boundary; only the part is copied.
    fn next_part(&mut self) -> String {
        let end = if self.text.len().saturating_sub(self.at) <= MAX_SEND_BYTES {
            self.text.len()
        } else {
            let mut cut = self.at.saturating_add(MAX_SEND_BYTES);
            while cut > self.at && !self.text.is_char_boundary(cut) {
                cut = cut.saturating_sub(1);
            }
            cut
        };
        let part = self.text.get(self.at..end).unwrap_or_default().to_owned();
        self.at = end;
        part
    }

    /// Whether every part went out.
    const fn is_spent(&self) -> bool {
        self.at >= self.text.len()
    }
}
