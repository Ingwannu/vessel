//! A machine's catalog watcher: read the catalog, long-poll `plugin.watch`
//! from its generation, read again when it moves (the page's
//! `followPlugins`).
//!
//! [Decision Log]
//! - Purpose: keep each serving machine's catalog current for the palette
//!   and the composer through busy refusals, timeouts and a node that
//!   answers at once.
//! - Constraint: the model has no clock and sends nothing by itself; the
//!   app sends its calls and keeps its timers.
//! - Choice (app review notes 1 and 2):
//!   - A failed read or watch waits [`RETRY_AFTER`] and reads again within
//!     the same connection. The first version stopped until the machine
//!     connected again, so one busy refusal or a read timing out at node
//!     start left the items stale for the whole connection.
//!   - A watch that brought nothing new waits [`QUIET_AFTER`] before
//!     watching again, so a node that answers at once (it is stopping) is
//!     not asked at round-trip speed.
//!   - A wait is an [`Answered::Paused`] that the app resumes after its
//!     timer. The wait's ticket becomes the next call's, so a wait that
//!     outlived its connection (a new epoch, the machine gone) resumes to
//!     nothing.
//! - Difference from the page: the page measures its quiet wait from when
//!   it asked (pace.ts `quiet`); this one waits after the answer, which
//!   adds at most a second after each 25 s hold and loses no change (the
//!   next watch answers at once if the generation moved meanwhile).
//! - Cost: one long poll per connected serving machine; after a failure,
//!   one read every 5 s until it answers.

use std::time::Duration;

use vessel_proto::RawJson;
use vessel_proto::plugins::PluginGeneration;

use crate::board::{Fleet, Reach};
use crate::connection::ConnectionEpoch;
use crate::devices::{MachineKey, Ticket};

use super::super::{Call, Problem, Reply, Request};
use super::{Catalogs, Held, PluginSlash, speaks_catalog};

/// The wait after a failed call before reading again (the page's
/// `RETRY_MS`).
pub const RETRY_AFTER: Duration = Duration::from_secs(5);

/// The wait after a watch that brought nothing new (the page's
/// `QUIET_MS`).
pub const QUIET_AFTER: Duration = Duration::from_secs(1);

/// A machine's catalog watcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Watch {
    /// The catalog is being read.
    Reading(Ticket),
    /// The watch is open after the generation the held catalog was read at.
    Watching(Ticket, PluginGeneration),
    /// Waiting to ask again under this ticket: watch after the generation,
    /// or with none, read the catalog again.
    Waiting(Ticket, Option<PluginGeneration>),
    /// Every answer identity was used; it waits for the next connection.
    Stopped,
}

/// The watcher of one connection to a machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Watcher {
    /// The connection it belongs to.
    epoch: Option<ConnectionEpoch>,
    /// Where it is.
    watch: Watch,
}

/// A watcher's wait, to resume with [`Catalogs::resume`] after its time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pause {
    machine: MachineKey,
    ticket: Ticket,
}

/// What an answer came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answered {
    /// Not one of these calls, or a late one: nothing changed.
    Stale,
    /// The watcher moved on: send these calls. `redraw` when a catalog
    /// changed.
    Asked {
        /// What to send.
        calls: Vec<(MachineKey, Call)>,
        /// Whether what the catalogs offer changed.
        redraw: bool,
    },
    /// The watcher waits `wait`, then resumes with `pause`.
    Paused {
        /// What to resume.
        pause: Pause,
        /// How long to wait first.
        wait: Duration,
    },
    /// A plugin method call came back: its output, or why not.
    Invoked(Result<RawJson, Problem>),
}

/// What a watcher does after an answer.
#[derive(Clone, Copy)]
enum Next {
    /// Read the catalog.
    Read,
    /// Watch after the generation.
    Watch(PluginGeneration),
    /// Wait, then watch after the generation or, with none, read.
    Wait(Option<PluginGeneration>),
}

impl Catalogs {
    /// Follow the fleet: forget the machines out of the book, end the
    /// watchers of machines that stopped serving, and start one for each
    /// connection that serves the catalog and has none. Returns the reads
    /// to send.
    pub fn sync(&mut self, fleet: &Fleet) -> Vec<(MachineKey, Call)> {
        self.book = fleet
            .machines
            .iter()
            .map(|machine| MachineKey::from_book(&machine.name))
            .collect();
        let book = &self.book;
        self.held.retain(|machine, _held| book.contains(machine));
        let mut calls = Vec::new();
        for machine in &fleet.machines {
            let key = MachineKey::from_book(&machine.name);
            let serving = machine.reach == Reach::Online && speaks_catalog(machine.protocol);
            let current = self
                .watchers
                .get(&key)
                .is_some_and(|watcher| watcher.epoch == machine.connection_epoch);
            if !serving {
                let _ended = self.watchers.remove(&key);
            } else if !current {
                let Ok(ticket) = self.tickets.next_ticket() else {
                    continue;
                };
                let watcher = Watcher {
                    epoch: machine.connection_epoch,
                    watch: Watch::Reading(ticket),
                };
                let _replaced = self.watchers.insert(key.clone(), watcher);
                calls.push((
                    key,
                    Call {
                        ticket,
                        request: Request::Catalog,
                    },
                ));
            }
        }
        let book = &self.book;
        self.watchers
            .retain(|machine, _watcher| book.contains(machine));
        calls
    }

    /// Take `machine`'s answer to `ticket`.
    pub fn answered(
        &mut self,
        machine: &MachineKey,
        ticket: Ticket,
        result: Result<Reply, Problem>,
    ) -> Answered {
        if let Some(at) = self.invoking.iter().position(|held| *held == ticket) {
            let _done = self.invoking.swap_remove(at);
            return Answered::Invoked(result.and_then(Reply::into_output));
        }
        let Some(watcher) = self.watchers.get(machine).copied() else {
            return Answered::Stale;
        };
        let (next, redraw) = match (watcher.watch, result) {
            (Watch::Reading(asked), Ok(Reply::Catalog(catalog))) if asked == ticket => {
                let generation = catalog.generation;
                let slash = PluginSlash::declared(machine, &catalog);
                let _earlier = self.held.insert(machine.clone(), Held { catalog, slash });
                (Next::Watch(generation), true)
            }
            (Watch::Watching(asked, seen), Ok(Reply::Generation(now))) if asked == ticket => {
                let next = if now == seen {
                    Next::Wait(Some(seen))
                } else {
                    Next::Read
                };
                (next, false)
            }
            (Watch::Reading(asked) | Watch::Watching(asked, _), _) if asked == ticket => {
                (Next::Wait(None), false)
            }
            (Watch::Reading(_) | Watch::Watching(..) | Watch::Waiting(..) | Watch::Stopped, _) => {
                return Answered::Stale;
            }
        };
        let Ok(ticket) = self.tickets.next_ticket() else {
            self.watch(machine, watcher.epoch, Watch::Stopped);
            return Answered::Asked {
                calls: Vec::new(),
                redraw,
            };
        };
        let (watch, request) = match next {
            Next::Read => (Watch::Reading(ticket), Request::Catalog),
            Next::Watch(after) => (Watch::Watching(ticket, after), Request::Watch(after)),
            Next::Wait(seen) => {
                self.watch(machine, watcher.epoch, Watch::Waiting(ticket, seen));
                return Answered::Paused {
                    pause: Pause {
                        machine: machine.clone(),
                        ticket,
                    },
                    wait: if seen.is_some() {
                        QUIET_AFTER
                    } else {
                        RETRY_AFTER
                    },
                };
            }
        };
        self.watch(machine, watcher.epoch, watch);
        Answered::Asked {
            calls: vec![(machine.clone(), Call { ticket, request })],
            redraw,
        }
    }

    /// After a wait: the call it waited to send, or nothing when the
    /// watcher has moved on (a new connection, or the machine is gone).
    pub fn resume(&mut self, pause: &Pause) -> Vec<(MachineKey, Call)> {
        let Some(watcher) = self.watchers.get_mut(&pause.machine) else {
            return Vec::new();
        };
        let Watch::Waiting(ticket, seen) = watcher.watch else {
            return Vec::new();
        };
        if ticket != pause.ticket {
            return Vec::new();
        }
        let (watch, request) = match seen {
            Some(after) => (Watch::Watching(ticket, after), Request::Watch(after)),
            None => (Watch::Reading(ticket), Request::Catalog),
        };
        watcher.watch = watch;
        vec![(pause.machine.clone(), Call { ticket, request })]
    }

    /// `machine`'s watcher, of the connection `epoch`, is now at `watch`.
    fn watch(&mut self, machine: &MachineKey, epoch: Option<ConnectionEpoch>, watch: Watch) {
        let _moved = self
            .watchers
            .insert(machine.clone(), Watcher { epoch, watch });
    }
}
