//! Telling a key the system repeats from a new press.
//!
//! [Decision Log]
//! - Purpose: GOAT's queue key (send the first queued message now,
//!   `thread.steerQueuedMessage`) ignores the system's key repeat, so a
//!   held Mod+Shift+Enter sends one message, not one every 30 ms.
//! - Constraint: the toolkit's keystroke interceptor, which the app's one
//!   listener uses so a shortcut can take a key before a field acts on it,
//!   is not told whether a press repeats; the raw key event that is runs
//!   after it.
//! - Choice: remember the chord last pressed until any key comes up; a
//!   press of the same chord before that is a repeat. Releasing the key
//!   (or a modifier) and pressing again is a new press.
//! - Cost: the shell's root hears every key-up to clear it (one line in
//!   the shell); a key-up the window never sees (focus lost while held)
//!   leaves the next identical press counted as a repeat once.

use super::keys::Chord;

/// The chord held down, if any.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Held(Option<Chord>);

impl Held {
    /// A press of `chord`: whether it is the system repeating the one held.
    pub fn press(&mut self, chord: &Chord) -> bool {
        let repeat = self.0.as_ref() == Some(chord);
        self.0 = Some(chord.clone());
        repeat
    }

    /// Any key came up: the next press is new.
    pub fn release(&mut self) {
        self.0 = None;
    }
}
