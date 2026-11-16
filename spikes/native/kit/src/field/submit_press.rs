//! Synchronous metadata from the existing `PressEnter` callback, not another listener.
//!
//! Enter's modifier chooses an alternate composer action only after the caller
//! applies its send setting and current admission. It cannot be guessed later
//! from global window modifiers, which may have changed before a queued callback.
use super::TextField;
use std::{cell::Cell, rc::Rc};

/// The accepted engine press while the existing submit callback runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitPress {
    /// Unmodified Enter.
    Enter,
    /// Command/Control+Enter; the send setting decides its meaning.
    ModifiedEnter,
}

impl TextField {
    /// Only available synchronously inside this field's existing submit callback.
    /// Buttons and later asynchronous work receive None. Event adapters must copy
    /// this value into their own typed event before returning from the callback.
    #[must_use]
    pub fn submit_press(&self) -> Option<SubmitPress> {
        self.submit_press.get()
    }
}

pub(super) struct Scope {
    cell: Rc<Cell<Option<SubmitPress>>>,
    previous: Option<SubmitPress>,
}
impl Scope {
    pub(super) fn enter(cell: &Rc<Cell<Option<SubmitPress>>>, secondary: bool) -> Self {
        let press = if secondary {
            SubmitPress::ModifiedEnter
        } else {
            SubmitPress::Enter
        };
        Self {
            previous: cell.replace(Some(press)),
            cell: Rc::clone(cell),
        }
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        self.cell.set(self.previous);
    }
}

#[cfg(test)]
mod tests;
