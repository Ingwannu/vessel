//! Whether a sequence is open, and whether it began under `replay`: the
//! hand-over rule of ADR 0073 (a sequence whose introducer was consumed
//! by `replay` never answers, even when it ends in live bytes).
//!
//! `vte` does not say what state it is in, so this follows the same
//! transitions for the one thing the rule needs: inside a sequence or
//! not. It mirrors `vte`'s table where that matters (ESC opens; CAN and
//! SUB abort; a CSI or escape ends at its final byte; an OSC at BEL or
//! ST; a DCS and SOS/PM/APC at ST) and ignores parameters and UTF-8.

use super::Source;

/// Where the bytes are in the sequence grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    /// After ESC.
    Escape,
    /// After ESC and an intermediate byte.
    Intermediate,
    /// Inside `ESC [`.
    Csi,
    /// Inside `ESC ]`.
    Osc,
    /// Inside `ESC P`, its parameters or its data.
    Dcs,
    /// Inside SOS, PM or APC (`ESC X`, `ESC ^`, `ESC _`): dropped.
    Text,
}

/// The tracker: the state, and where the open sequence began.
#[derive(Debug, Clone, Copy)]
pub(super) struct Track {
    state: State,
    began: Source,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            state: State::Ground,
            began: Source::Live,
        }
    }
}

impl Track {
    /// Whether a sequence is open that began under `replay`.
    pub(super) fn open_from_replay(self) -> bool {
        self.state != State::Ground && self.began == Source::Replay
    }

    /// Follow `bytes`, read under `source`.
    pub(super) fn scan(&mut self, bytes: &[u8], source: Source) {
        for byte in bytes {
            self.step(*byte, source);
        }
    }

    /// Follow one byte.
    pub(super) fn step(&mut self, byte: u8, source: Source) {
        const ESC: u8 = 0x1b;
        // CAN and SUB abort any sequence; ESC starts a new one anywhere
        // (in an OSC, DCS or string it is the first byte of ST, whose
        // `\` ends it from the escape state).
        match byte {
            0x18 | 0x1a => {
                self.state = State::Ground;
                return;
            }
            ESC => {
                self.state = State::Escape;
                self.began = source;
                return;
            }
            _ => {}
        }
        // The introducers first: they are final bytes too.
        self.state = match (self.state, byte) {
            (State::Escape, b'[') => State::Csi,
            (State::Escape, b']') => State::Osc,
            (State::Escape, b'P') => State::Dcs,
            (State::Escape, b'X' | b'^' | b'_') => State::Text,
            (State::Escape | State::Intermediate, 0x20..=0x2f) => State::Intermediate,
            // Outside a sequence; an escape's or a CSI's final byte; BEL
            // ending an OSC.
            (State::Ground, _)
            | (State::Escape | State::Intermediate, 0x30..=0x7e)
            | (State::Csi, 0x40..=0x7e)
            | (State::Osc, 0x07) => State::Ground,
            // C0 controls are executed inside a sequence and keep it open;
            // DEL and the rest are ignored or collected.
            (
                state @ (State::Escape
                | State::Intermediate
                | State::Csi
                | State::Osc
                | State::Dcs
                | State::Text),
                _,
            ) => state,
        };
    }
}
