//! One worker at a time applies the "Terminal agent hooks" setting, always
//! the latest state wanted.
//!
//! [Decision Log]
//! - Purpose: each apply reads a person's file, changes it and writes it
//!   back. Two applies at once (a quick on then off, or a change during the
//!   one at start) could interleave: "on" writing after "off" read, leaving
//!   the hooks installed while the setting says off (node review R3).
//! - Paseo applies on its event loop, one at a time
//!   (`terminal-agent-hook-setting.ts`); Vessel's file work runs on the
//!   blocking pool, so it needs its own order.
//! - Choice: a single-flight worker. A change records the state wanted and
//!   starts the worker unless one runs. The worker applies the latest
//!   wanted state, then looks again, and stops when nothing new is wanted.
//!   Only the last state matters, so states wanted while it worked are
//!   folded into one pass.
//! - Alternatives: a lock around each apply (still runs every stale state
//!   in turn, and the order the blocking pool picks them is not the order
//!   asked); a channel with every change (the same).
//! - Cost: a state wanted while a pass runs waits for that pass.

use std::sync::{Arc, Mutex, PoisonError};

use tokio_util::task::TaskTracker;

use super::Targets;

/// The state wanted and whether a worker applies it.
#[derive(Debug, Default)]
struct Wanted {
    /// The latest state asked for and not yet taken by the worker.
    next: Option<bool>,
    /// A worker runs; it takes `next` before it stops.
    running: bool,
}

/// Applies the setting through one worker at a time.
#[derive(Debug)]
pub(crate) struct Applier {
    targets: Targets,
    wanted: Mutex<Wanted>,
}

impl Applier {
    /// For the agents' homes in `targets`.
    pub(crate) fn new(targets: Targets) -> Arc<Self> {
        Arc::new(Self {
            targets,
            wanted: Mutex::new(Wanted::default()),
        })
    }

    /// Want `on`: the worker on `tasks` applies it, after the pass it may be
    /// in.
    pub(crate) fn want(self: &Arc<Self>, on: bool, tasks: &TaskTracker) {
        let start = {
            let mut wanted = self.wanted.lock().unwrap_or_else(PoisonError::into_inner);
            wanted.next = Some(on);
            !std::mem::replace(&mut wanted.running, true)
        };
        if start {
            let applier = Arc::clone(self);
            drop(tasks.spawn_blocking(move || applier.work()));
        }
    }

    /// Apply the latest wanted state until none is new.
    fn work(&self) {
        loop {
            let next = {
                let mut wanted = self.wanted.lock().unwrap_or_else(PoisonError::into_inner);
                let next = wanted.next.take();
                if next.is_none() {
                    wanted.running = false;
                }
                next
            };
            match next {
                Some(on) => self.targets.apply(on),
                None => return,
            }
        }
    }
}
