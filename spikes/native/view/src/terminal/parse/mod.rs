//! The decoding edge of the terminal grid (ADR 0073): a program's bytes
//! in, typed actions out, each tagged with whether its sequence began
//! under `replay` or live.
//!
//! [Decision Log]
//! - Purpose: the grid applies a program's output; the program is on the
//!   other side of the node and its bytes are untrusted. This module is
//!   where they become types, and where every buffer is bounded.
//! - Constraint: `vte` 0.15 is the state machine (ADR 0073). In its `std`
//!   build an OSC string is kept in a `Vec` that grows as long as the
//!   program writes one (`lib.rs` 63–64, 547–551); without `std` it is an
//!   `ArrayVec` of a const size, and bytes past it are dropped.
//! - Choice: `vte` without default features: OSC strings are capped at
//!   [`OSC_LIMIT`] inside the parser, parameters at its 32, intermediates
//!   at its 2. A DCS's data is never kept (each byte is dropped as it
//!   comes). One call decodes at most [`FEED_BUDGET`] bytes and so emits
//!   at most that many actions. The hand-over rule (a sequence begun in
//!   `replay` never answers) is decided here, byte by byte while such a
//!   sequence is open, by [`track::Track`].
//! - Cost: `vte`'s `memchr` fast path for printable runs is only in its
//!   `std` build; plain text is decoded byte by byte. A title longer than
//!   1,024 bytes is cut.

mod action;
mod decode;
mod track;

#[cfg(test)]
mod tests;

pub use action::{
    Action, Attributes, ColourSlot, Control, Count, Csi, Dcs, Erase, Esc, Group, Mode, Osc,
    Position, Status, Switch, TabClear,
};

use vte::{Params, Perform};

/// The most bytes one call decodes: the PTY's read (ADR 0073).
pub const FEED_BUDGET: usize = 16 * 1024;

/// The most bytes of one OSC string kept (`vte`'s const buffer).
pub const OSC_LIMIT: usize = 1024;

/// How bytes are applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// History, a fold of the ring, or a viewer that must not answer.
    Replay,
    /// Live output a viewer may answer.
    Live,
}

/// An action and where its sequence began: an action begun under
/// `Replay` never answers, wherever it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// What the program asked.
    pub action: Action,
    /// Where its sequence began.
    pub began: Source,
}

/// The program's byte stream, decoded across calls (a sequence may span
/// calls).
#[derive(Default)]
pub struct Decoder {
    parser: vte::Parser<OSC_LIMIT>,
    track: track::Track,
    /// What the DCS being read asks (from its hook; a DCS may span calls).
    dcs: Dcs,
}

impl std::fmt::Debug for Decoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Decoder")
            .field("track", &self.track)
            .field("dcs", &self.dcs)
            .finish_non_exhaustive()
    }
}

impl Decoder {
    /// A decoder at the start of a stream.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode at most [`FEED_BUDGET`] bytes of `bytes`, read under
    /// `source`, appending what they mean to `out`. Returns how many bytes
    /// were taken; the caller keeps the rest for the next call.
    pub fn decode(&mut self, bytes: &[u8], source: Source, out: &mut Vec<Decoded>) -> usize {
        let taken = bytes.len().min(FEED_BUDGET);
        let bytes = bytes.get(..taken).unwrap_or_default();
        let mut rest = bytes;
        // Live bytes that finish a sequence begun in replay: one at a
        // time, tagged `Replay`, until that sequence is over.
        while source == Source::Live && self.track.open_from_replay() {
            let Some((byte, after)) = rest.split_first() else {
                break;
            };
            let mut sink = Sink {
                out: &mut *out,
                began: Source::Replay,
                dcs: &mut self.dcs,
            };
            self.parser.advance(&mut sink, std::slice::from_ref(byte));
            self.track.step(*byte, source);
            rest = after;
        }
        let mut sink = Sink {
            out,
            began: source,
            dcs: &mut self.dcs,
        };
        self.parser.advance(&mut sink, rest);
        self.track.scan(rest, source);
        taken
    }
}

/// `vte`'s callbacks, typed and tagged.
struct Sink<'a> {
    out: &'a mut Vec<Decoded>,
    began: Source,
    /// What the DCS being read asks (the decoder's, across calls).
    dcs: &'a mut Dcs,
}

impl Sink<'_> {
    fn push(&mut self, action: Action) {
        self.out.push(Decoded {
            action,
            began: self.began,
        });
    }
}

impl Perform for Sink<'_> {
    fn print(&mut self, c: char) {
        self.push(Action::Print(c));
    }

    fn execute(&mut self, byte: u8) {
        self.push(Action::Control(Control::of(byte)));
    }

    fn hook(&mut self, _params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        *self.dcs = decode::dcs(intermediates, ignore, action);
    }

    // A DCS's data is never kept: nothing in it is drawn or answered.
    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {
        let kind = *self.dcs;
        self.push(Action::Dcs(kind));
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        self.push(Action::Osc(decode::osc(params)));
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.push(Action::Csi(decode::csi(
            params,
            intermediates,
            ignore,
            action,
        )));
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        self.push(Action::Esc(decode::esc(intermediates, ignore, byte)));
    }
}
