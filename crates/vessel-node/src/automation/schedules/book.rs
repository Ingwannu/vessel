//! The schedules as the scheduler keeps them, and the order they come due
//! in. Owns: each schedule's info and whether a run of it is being made;
//! the index of due times. Must not: read a clock, write the store or make
//! a run (the actor does, `actor`).
//!
//! The index is what keeps a wake cheap (the contract's "Time"): the
//! scheduler looks at its first entry, never at every schedule. Every
//! change of a schedule's state goes through [`Book::put`], which moves its
//! index entry with it, so the two cannot disagree.

use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, Utc};
use vessel_proto::automation::{ScheduleInfo, ScheduleState};
use vessel_proto::{Creator, ScheduleId, SessionId};

/// A schedule and what the scheduler knows of it besides.
#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub(super) info: ScheduleInfo,
    /// A run of it is being made (from its `making` record to its
    /// outcome): a due time meanwhile is skipped, one run at a time.
    pub(super) making: bool,
}

/// Every schedule, and the active ones by due time.
#[derive(Debug, Default)]
pub(super) struct Book {
    entries: HashMap<ScheduleId, Entry>,
    due: BTreeSet<(DateTime<Utc>, ScheduleId)>,
}

/// When `state` is next due, if it is active.
fn due_of(state: &ScheduleState) -> Option<DateTime<Utc>> {
    match state {
        ScheduleState::Active { next_due } => Some(*next_due),
        ScheduleState::Paused { .. } | ScheduleState::Ended { .. } | ScheduleState::Unknown => None,
    }
}

impl Book {
    /// Insert or replace a schedule, its index entry moved with it; a run
    /// being made stays so.
    pub(super) fn put(&mut self, info: ScheduleInfo) {
        let id = info.id;
        let making = match self.entries.get(&id) {
            Some(old) => {
                if let Some(due) = due_of(&old.info.state) {
                    let _was = self.due.remove(&(due, id));
                }
                old.making
            }
            None => false,
        };
        if let Some(due) = due_of(&info.state) {
            let _new = self.due.insert((due, id));
        }
        drop(self.entries.insert(id, Entry { info, making }));
    }

    /// Mark whether a run of `id` is being made.
    pub(super) fn set_making(&mut self, id: ScheduleId, making: bool) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.making = making;
        }
    }

    /// Take a schedule out.
    pub(super) fn remove(&mut self, id: ScheduleId) -> Option<Entry> {
        let entry = self.entries.remove(&id)?;
        if let Some(due) = due_of(&entry.info.state) {
            let _was = self.due.remove(&(due, id));
        }
        Some(entry)
    }

    pub(super) fn get(&self, id: ScheduleId) -> Option<&Entry> {
        self.entries.get(&id)
    }

    /// The earliest due time of an active schedule.
    pub(super) fn first_due(&self) -> Option<DateTime<Utc>> {
        self.due.first().map(|(due, _id)| *due)
    }

    /// The schedule due earliest, if its due time is at or before `now`.
    /// It stays in the index until its state changes (`put`).
    pub(super) fn next_due_by(&self, now: DateTime<Utc>) -> Option<ScheduleId> {
        self.due
            .first()
            .filter(|(due, _id)| *due <= now)
            .map(|(_due, id)| *id)
    }

    /// Every schedule, newest first.
    pub(super) fn list(&self) -> Vec<ScheduleInfo> {
        let mut all: Vec<ScheduleInfo> = self
            .entries
            .values()
            .map(|entry| entry.info.clone())
            .collect();
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
        all
    }

    /// Every schedule's id and info, for a pass over all (at start, and
    /// when the clock was found stepped back: never at a wake).
    pub(super) fn all(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }

    /// How many schedules people made, and how many agents made.
    pub(super) fn counts(&self) -> (usize, usize) {
        let agents = self
            .entries
            .values()
            .filter(|entry| matches!(entry.info.defined_by, Creator::Agent { .. }))
            .count();
        (self.entries.len().saturating_sub(agents), agents)
    }

    /// The schedules an agent's session defined.
    pub(super) fn of_agent(&self, session: SessionId) -> Vec<ScheduleId> {
        self.entries
            .values()
            .filter(|entry| {
                matches!(entry.info.defined_by, Creator::Agent { session: by, .. } if by == session)
            })
            .map(|entry| entry.info.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;
    use vessel_proto::automation::{
        Cadence, CatchUp, EverySeconds, ScheduleDefinition, ScheduleTarget,
    };

    use super::*;

    fn info(next_due: Option<DateTime<Utc>>) -> ScheduleInfo {
        let at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        ScheduleInfo {
            id: ScheduleId::random(),
            definition: ScheduleDefinition {
                name: None,
                cadence: Cadence::Every {
                    seconds: EverySeconds::try_from(60).unwrap(),
                    anchor: Some(at),
                },
                target: ScheduleTarget::Session {
                    session: SessionId::random(),
                },
                prompt: "Go.".parse().unwrap(),
                catch_up: CatchUp::Once,
                max_runs: None,
                ends_at: None,
            },
            state: next_due.map_or(ScheduleState::Paused { since: at }, |next_due| {
                ScheduleState::Active { next_due }
            }),
            runs: 0,
            last_due: None,
            last_run: None,
            defined_by: Creator::Owner,
            created_at: at,
            updated_at: at,
            revision: 0,
        }
    }

    #[test]
    fn due_ones_come_in_due_order_then_by_id_and_move_with_their_state() {
        let ten = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let eleven = Utc.with_ymd_and_hms(2026, 1, 1, 11, 0, 0).unwrap();
        let mut book = Book::default();
        let (mut a, b, late, paused) = (
            info(Some(ten)),
            info(Some(ten)),
            info(Some(eleven)),
            info(None),
        );
        for schedule in [&late, &a, &b, &paused] {
            book.put(schedule.clone());
        }
        assert_eq!(book.first_due(), Some(ten));
        let first = a.id.min(b.id);
        assert_eq!(book.next_due_by(ten), Some(first));
        assert_eq!(book.next_due_by(ten - chrono::TimeDelta::seconds(1)), None);
        // Moving one on takes its old place out of the index.
        a.state = ScheduleState::Active { next_due: eleven };
        book.put(a.clone());
        assert_eq!(book.next_due_by(ten), Some(b.id));
        drop(book.remove(b.id));
        assert_eq!(book.first_due(), Some(eleven));
        assert_eq!(book.next_due_by(ten), None);
        // A paused one is never due.
        assert_eq!(book.list().len(), 3);
        assert_eq!(book.counts(), (3, 0));
    }
}
