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
//! - Bound: at most `MAX_PENDING` characters of keys wait; past that,
//!   keys are dropped and the screen is told once per overflow (an
//!   unbounded queue behind a harness that stopped reading would grow
//!   without end).
//! - Failures: a send that fails is not repeated (it may have been
//!   applied); later items still go, in order.

use std::collections::VecDeque;

/// Most characters of keys waiting while a send is in flight.
pub const MAX_PENDING: usize = 1024 * 1024;

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

/// The sender's queue.
#[derive(Debug, Default)]
pub struct TerminalSender {
    items: VecDeque<Outgoing>,
    chars: usize,
    busy: bool,
    closed: bool,
    overflowing: bool,
}

impl TerminalSender {
    /// Queue keys, joined to keys queued just before.
    pub fn keys(&mut self, text: &str) -> Taken {
        if self.closed || text.is_empty() {
            return Taken::Queued;
        }
        let count = text.chars().count();
        if self.chars.saturating_add(count) > MAX_PENDING {
            if self.overflowing {
                return Taken::Dropped;
            }
            self.overflowing = true;
            return Taken::DroppedFirst;
        }
        self.overflowing = false;
        self.chars = self.chars.saturating_add(count);
        if let Some(Outgoing::Keys(last)) = self.items.back_mut() {
            last.push_str(text);
        } else {
            self.items.push_back(Outgoing::Keys(text.to_owned()));
        }
        Taken::Queued
    }

    /// Queue a size; a size queued just before is replaced.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if self.closed {
            return;
        }
        if let Some(Outgoing::Resize {
            cols: last_cols,
            rows: last_rows,
        }) = self.items.back_mut()
        {
            *last_cols = cols;
            *last_rows = rows;
        } else {
            self.items.push_back(Outgoing::Resize { cols, rows });
        }
    }

    /// The next send, when none is in flight: it is then in flight until
    /// `answered`.
    pub fn take(&mut self) -> Option<Outgoing> {
        if self.busy || self.closed {
            return None;
        }
        let item = self.items.pop_front()?;
        if let Outgoing::Keys(text) = &item {
            self.chars = self.chars.saturating_sub(text.chars().count());
        }
        self.busy = true;
        Some(item)
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
    }

    /// Whether a send is in flight.
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.busy
    }
}

#[cfg(test)]
mod tests;
