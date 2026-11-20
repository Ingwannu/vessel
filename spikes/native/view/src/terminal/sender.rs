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
//!   node refuses a larger one whole.
//! - Failures: a send that fails is not repeated (it may have been
//!   applied); later items still go, in order.
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
    Typed(String),
    Pasted(String),
    Resize {
        cols: u16,
        rows: u16,
        intent: ResizeIntent,
    },
}

/// The sender's queue.
#[derive(Debug, Default)]
pub struct TerminalSender {
    items: VecDeque<Item>,
    /// Characters of typed keys queued.
    chars: usize,
    /// Bytes of pastes queued.
    pasted: usize,
    busy: bool,
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
            last.push_str(text);
        } else {
            self.items.push_back(Item::Typed(text.to_owned()));
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
        self.items.push_back(Item::Pasted(encoded));
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
        if self.busy || self.closed {
            return None;
        }
        let outgoing = match self.items.front_mut()? {
            Item::Typed(text) => {
                let part = front_part(text);
                self.chars = self.chars.saturating_sub(part.chars().count());
                Outgoing::Keys(part)
            }
            Item::Pasted(text) => {
                let part = front_part(text);
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
            Some(Item::Typed(text) | Item::Pasted(text)) => text.is_empty(),
            Some(Item::Resize { .. }) | None => true,
        };
        if spent {
            let _sent = self.items.pop_front();
        }
        self.busy = true;
        Some(outgoing)
    }

    /// The send in flight answered, well or not (a failed send is not
    /// repeated). The next one may go.
    pub const fn answered(&mut self) {
        self.busy = false;
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
        self.busy
    }
}

/// Take the front of `text` up to one send's bound, cut at a character
/// boundary; `text` keeps the rest.
fn front_part(text: &mut String) -> String {
    if text.len() <= MAX_SEND_BYTES {
        return std::mem::take(text);
    }
    let mut cut = MAX_SEND_BYTES;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut = cut.saturating_sub(1);
    }
    let rest = text.split_off(cut);
    std::mem::replace(text, rest)
}
