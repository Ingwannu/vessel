//! Room for runs (ADR 0033): which runs are being made, which wait, and in
//! what order they are let go. Pure: the actor asks it and acts; the tests
//! ask it without one.
//!
//! [Decision Log]
//! - Purpose: the contract's "at most 4 runs are made at once; the rest
//!   wait, oldest due first, then by id", and a bound on what a catch-up
//!   after downtime starts at once.
//! - Constraint: track D's run tasks each waited on a semaphore, so the
//!   order they were let go in was the runtime's (tasks of one actor start
//!   in any order on a multi-threaded runtime), and a slot freed at the
//!   run's outcome bounded how fast new sessions started, not how many
//!   were in their first turn at once: after a night off, every schedule
//!   of new sessions (up to 256) would start one within seconds (the
//!   join's code review, finding 12).
//! - Alternatives: a fair semaphore taken in each task (the same race);
//!   the actor awaiting a permit (it would stop answering calls while it
//!   waits); spreading a catch-up over time (a run made later than it
//!   needs to be, and a rule of its own to explain).
//! - Choice: the actor hands out room itself, without waiting. A run
//!   waits here, keyed by its due time and its schedule, until a slot is
//!   free and, for a run that starts a session, a first turn: at most
//!   [`TURNS`] scheduled sessions are in their first turn and not waiting
//!   for a person at once. The first that fits goes first, so a prompt
//!   through a queue is not held behind sessions waiting for a turn. An
//!   asked run (`schedule.run`) waits [`super::RUN_NOW_WAIT`] at most, on the
//!   scheduler's clock, as due times are.
//! - Cost: the actor counts room and wakes for an asked run's deadline;
//!   a run that starts a session can wait behind long first turns, its due
//!   time recorded as it was.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use vessel_proto::ScheduleId;

/// Scheduled sessions in their first turn at once.
pub(crate) const TURNS: usize = 8;

/// What a waiting run needs besides a slot, and how long it may wait.
#[derive(Debug)]
struct Waiting<T> {
    item: T,
    /// It starts a session, which takes a first turn.
    starts: bool,
    /// An asked run's: past it, the run is not made.
    deadline: Option<DateTime<Utc>>,
}

/// The room and the runs waiting for it.
#[derive(Debug)]
pub(super) struct Room<T> {
    slots: usize,
    turns: usize,
    waiting: BTreeMap<(DateTime<Utc>, ScheduleId), Waiting<T>>,
}

/// A run let go, and whether it took a first turn.
#[derive(Debug)]
pub(super) struct Let<T> {
    pub(super) item: T,
    pub(super) turn: bool,
}

impl<T> Room<T> {
    /// Room for `slots` runs at once and `turns` first turns.
    pub(super) const fn new(slots: usize, turns: usize) -> Self {
        Self {
            slots,
            turns,
            waiting: BTreeMap::new(),
        }
    }

    /// A run for `due_at` of `schedule` waits. A schedule has one run being
    /// made at a time, so the key is unique.
    pub(super) fn wait(
        &mut self,
        due_at: DateTime<Utc>,
        schedule: ScheduleId,
        item: T,
        starts: bool,
        deadline: Option<DateTime<Utc>>,
    ) {
        let _replaced = self.waiting.insert(
            (due_at, schedule),
            Waiting {
                item,
                starts,
                deadline,
            },
        );
    }

    /// The first waiting run that has room, oldest due first, then by
    /// schedule; its slot and its first turn taken.
    pub(super) fn next(&mut self) -> Option<Let<T>> {
        if self.slots == 0 {
            return None;
        }
        let turns = self.turns;
        let key = *self
            .waiting
            .iter()
            .find(|(_, waiting)| !waiting.starts || turns > 0)
            .map(|(key, _)| key)?;
        let waiting = self.waiting.remove(&key)?;
        self.slots = self.slots.saturating_sub(1);
        if waiting.starts {
            self.turns = self.turns.saturating_sub(1);
        }
        Some(Let {
            item: waiting.item,
            turn: waiting.starts,
        })
    }

    /// A run has its outcome: its slot is free.
    pub(super) const fn slot_freed(&mut self) {
        self.slots = self.slots.saturating_add(1);
    }

    /// A run's session ended its first turn, or waits for a person: its
    /// first turn is free.
    pub(super) const fn turn_freed(&mut self) {
        self.turns = self.turns.saturating_add(1);
    }

    /// The asked runs whose wait ended by `now`, taken out.
    pub(super) fn expired(&mut self, now: DateTime<Utc>) -> Vec<T> {
        let ended: Vec<_> = self
            .waiting
            .iter()
            .filter(|(_, waiting)| waiting.deadline.is_some_and(|deadline| deadline <= now))
            .map(|(key, _)| *key)
            .collect();
        ended
            .into_iter()
            .filter_map(|key| self.waiting.remove(&key))
            .map(|waiting| waiting.item)
            .collect()
    }

    /// The earliest deadline of an asked run still waiting.
    pub(super) fn next_deadline(&self) -> Option<DateTime<Utc>> {
        self.waiting
            .values()
            .filter_map(|waiting| waiting.deadline)
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(second: u32) -> DateTime<Utc> {
        DateTime::from_timestamp(i64::from(second), 0).unwrap()
    }

    fn ids(count: usize) -> Vec<ScheduleId> {
        let mut ids: Vec<_> = (0..count).map(|_| ScheduleId::random()).collect();
        ids.sort();
        ids
    }

    #[test]
    fn runs_go_oldest_due_first_then_by_schedule() {
        let ids = ids(3);
        let mut room = Room::new(1, TURNS);
        room.wait(at(20), ids[0], "late", false, None);
        room.wait(at(10), ids[2], "second of two", false, None);
        room.wait(at(10), ids[1], "first of two", false, None);
        let mut order = Vec::new();
        while let Some(run) = room.next() {
            order.push(run.item);
            room.slot_freed();
        }
        assert_eq!(order, ["first of two", "second of two", "late"]);
    }

    #[test]
    fn a_prompt_is_not_held_behind_sessions_waiting_for_a_first_turn() {
        let ids = ids(3);
        let mut room = Room::new(4, 1);
        room.wait(at(1), ids[0], "session", true, None);
        room.wait(at(2), ids[1], "another session", true, None);
        room.wait(at(3), ids[2], "prompt", false, None);
        let first = room.next().unwrap();
        assert_eq!((first.item, first.turn), ("session", true));
        // No first turn is free: the prompt goes, the session waits.
        assert_eq!(room.next().unwrap().item, "prompt");
        assert!(room.next().is_none());
        room.turn_freed();
        assert_eq!(room.next().unwrap().item, "another session");
    }

    #[test]
    fn at_most_the_slots_at_once_and_an_asked_run_waits_until_its_deadline() {
        let ids = ids(3);
        let mut room = Room::new(1, TURNS);
        room.wait(at(1), ids[0], "made", false, None);
        assert!(room.next().is_some());
        room.wait(at(2), ids[1], "asked", false, Some(at(32)));
        room.wait(at(3), ids[2], "due", false, None);
        assert!(room.next().is_none(), "one slot, taken");
        assert_eq!(room.next_deadline(), Some(at(32)));
        assert!(room.expired(at(31)).is_empty());
        assert_eq!(room.expired(at(32)), ["asked"]);
        assert_eq!(room.next_deadline(), None);
        room.slot_freed();
        assert_eq!(room.next().unwrap().item, "due");
    }
}
