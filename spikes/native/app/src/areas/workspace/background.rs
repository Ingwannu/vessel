//! The app's background file work on a machine (projects' icons, the Files
//! tree's Modified and Size facts) in one line per machine, a few at a
//! time; the page's `data/workspace/file-work.ts`.
//!
//! [Decision Log]
//! - Purpose: background asks must not be refused, nor crowd out what a
//!   person does.
//! - Constraint: the node runs file work (`file.inspect`, `project.icon`,
//!   `file.duplicate`, `file.read_bytes`, `file.suggest`, `folder.suggest`)
//!   on four slots and refuses a fifth at once as `busy`, without a queue.
//!   A sidebar of five projects, or the Files tree with four folders open
//!   switched to Modified, asked five at once: one was refused and its icon
//!   or its folder's order was lost.
//! - Alternatives: one line per caller (each still fills the slots beside
//!   the other); retrying `busy` (more calls into a full node, and the
//!   person's action can still lose); a queue on the node (a protocol
//!   change for what the app can pace itself).
//! - Choice: one line per machine shared by every background caller, at
//!   most [`BACKGROUND_SLOTS`] out, the rest waiting in order. Every ask
//!   gets exactly one answer (a command the link did not take is answered
//!   as `Unsent`), and the answer frees the slot. Two of the node's four
//!   stay free for a person's Duplicate, an image's bytes, a file search or
//!   Add project's folders.
//! - Cost: a long project list or many open folders fill in two at a time
//!   (each call is milliseconds on the node); another client of the same
//!   node (the page, a second app) is not counted. A machine renamed or
//!   forgotten keeps its old name's line until what was asked under it is
//!   answered (refused, as the name is gone); nothing waits on it.

use std::collections::{HashMap, VecDeque};

use vessel_app_link::workspace::Ask;

use crate::shell::Shell;

/// Background file work out at once per machine: below the node's four
/// file work slots, so a person's own file action keeps a slot.
pub(crate) const BACKGROUND_SLOTS: usize = 2;

/// Each machine's background asks out, and those waiting for a slot.
#[derive(Debug, Default)]
pub(crate) struct Background {
    out: HashMap<String, usize>,
    waiting: HashMap<String, VecDeque<Ask>>,
}

impl Background {
    /// `ask` to send now, taking a slot of `machine`'s, or `None` when it
    /// waits for one.
    fn take(&mut self, machine: &str, ask: Ask) -> Option<Ask> {
        let out = self.out.entry(machine.to_owned()).or_default();
        if *out < BACKGROUND_SLOTS {
            *out = out.saturating_add(1);
            return Some(ask);
        }
        self.waiting
            .entry(machine.to_owned())
            .or_default()
            .push_back(ask);
        None
    }

    /// A background ask of `machine`'s was answered: its slot goes to the
    /// next waiting ask (returned, to send), else is freed.
    fn give(&mut self, machine: &str) -> Option<Ask> {
        let next = self.waiting.get_mut(machine).and_then(VecDeque::pop_front);
        if next.is_none()
            && let Some(out) = self.out.get_mut(machine)
        {
            *out = out.saturating_sub(1);
        }
        next
    }

    /// `machine`'s asks out (tests).
    #[cfg(test)]
    pub(super) fn out(&self, machine: &str) -> usize {
        self.out.get(machine).copied().unwrap_or_default()
    }

    /// `machine`'s asks waiting for a slot (tests).
    #[cfg(test)]
    pub(super) fn waiting(&self, machine: &str) -> usize {
        self.waiting.get(machine).map_or(0, VecDeque::len)
    }
}

impl Shell {
    /// Send `ask` to `machine` on its background line: now when a slot is
    /// free, else when an earlier one is answered.
    pub(super) fn ask_in_background(&mut self, machine: &str, ask: Ask) {
        if let Some(ask) = self.areas.workspace.background.take(machine, ask) {
            self.ask(machine, ask);
        }
    }

    /// One of `machine`'s background asks was answered: the next goes out.
    pub(super) fn background_answered(&mut self, machine: &str) {
        if let Some(ask) = self.areas.workspace.background.give(machine) {
            self.ask(machine, ask);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_link::workspace::ProjectAsk;

    fn ask() -> Ask {
        Ask::Project(ProjectAsk::List)
    }

    #[test]
    fn a_machine_has_two_out_and_the_rest_wait_in_order() {
        let mut line = Background::default();
        assert!(line.take("laptop", ask()).is_some());
        assert!(line.take("laptop", ask()).is_some());
        assert!(line.take("laptop", ask()).is_none());
        assert!(line.take("laptop", ask()).is_none());
        // Another machine has its own line.
        assert!(line.take("build-server", ask()).is_some());
        assert_eq!((line.out("laptop"), line.waiting("laptop")), (2, 2));
        // Each answer hands its slot to the next waiting ask, then frees it.
        assert!(line.give("laptop").is_some());
        assert!(line.give("laptop").is_some());
        assert_eq!((line.out("laptop"), line.waiting("laptop")), (2, 0));
        assert!(line.give("laptop").is_none());
        assert!(line.give("laptop").is_none());
        assert_eq!(line.out("laptop"), 0);
        // An answer with nothing out changes nothing.
        assert!(line.give("laptop").is_none());
        assert_eq!(line.out("laptop"), 0);
        assert_eq!(line.out("build-server"), 1);
    }
}
