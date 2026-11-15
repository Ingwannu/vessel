//! The scheduler's calls (ADR 0033): list, read, make, change, pause,
//! resume, run now and delete a schedule, each decided by the rules of who
//! may (`rules`) and of when a schedule is due (`check`, `time`), inside the
//! scheduler's one task. Must not: make a run itself (`start_run` does,
//! in `actor`), nor read a session.

use tokio::sync::oneshot;
use vessel_proto::automation::{
    EndReason, RunTrigger, ScheduleDefinition, ScheduleInfo, ScheduleRun, ScheduleState,
};
use vessel_proto::{Creator, ScheduleId, SessionId};

use super::actor::Actor;
use super::book::Entry;
use super::check::{node_anchor, settle};
use super::rules::{AGENT_SCHEDULES, Act, Asker, PEOPLE_SCHEDULES};
use super::state::{advanced, ended_refused, ends_an_agents, update_admitted};
use super::{Clock, Command, Runner};
use crate::automation::error::AutomationError;
use crate::automation::time::Due;

impl<R: Runner, C: Clock> Actor<R, C> {
    pub(super) async fn handle(&mut self, command: Command) {
        match command {
            Command::List { reply } => {
                let _gone = reply.send(self.book.list());
            }
            Command::Get { schedule, reply } => {
                let found = self.found(schedule).map(|entry| entry.info.clone());
                let _gone = reply.send(found);
            }
            Command::Create {
                definition,
                paused,
                asker,
                reply,
            } => {
                let created = self.create(*definition, paused, asker).await;
                let _gone = reply.send(created);
            }
            Command::Update {
                schedule,
                revision,
                definition,
                asker,
                reply,
            } => {
                let updated = self.update(schedule, revision, *definition, &asker).await;
                let _gone = reply.send(updated);
            }
            Command::Pause {
                schedule,
                asker,
                reply,
            } => {
                let _gone = reply.send(self.pause(schedule, &asker, true).await);
            }
            Command::Resume {
                schedule,
                asker,
                reply,
            } => {
                let _gone = reply.send(self.pause(schedule, &asker, false).await);
            }
            Command::Delete {
                schedule,
                asker,
                reply,
            } => {
                let deleted = match self.found(schedule) {
                    Ok(entry) => asker.may(Act::Delete, &entry.info.defined_by),
                    Err(error) => Err(error),
                };
                if deleted.is_ok() {
                    self.forget(schedule).await;
                }
                let _gone = reply.send(deleted);
            }
            Command::Run {
                schedule,
                asker,
                reply,
            } => self.run_now(schedule, asker, reply).await,
            Command::StartingAgain { session, reply } => {
                self.session_ended(session).await;
                let _gone = reply.send(());
            }
        }
    }

    /// Some sessions' ends were missed (the scheduler fell behind their
    /// channel): every agent's schedule whose agent can no longer act goes
    /// now, rather than at a next run that a paused one never makes.
    pub(super) async fn sweep_agents(&mut self) {
        let gone: Vec<ScheduleId> = self
            .book
            .all()
            .filter(|entry| match &entry.info.defined_by {
                Creator::Agent { session, .. } => !self.runner.agent_alive(*session),
                Creator::Owner | Creator::Device { .. } => false,
            })
            .map(|entry| entry.info.id)
            .collect();
        for schedule in gone {
            self.forget(schedule).await;
        }
    }

    /// A session ended: the schedules its agent made go with it (ADR
    /// 0033: a session cannot use a schedule to outlive itself).
    pub(super) async fn session_ended(&mut self, session: SessionId) {
        for schedule in self.book.of_agent(session) {
            self.forget(schedule).await;
        }
    }

    fn found(&self, schedule: ScheduleId) -> Result<&Entry, AutomationError> {
        self.book
            .get(schedule)
            .ok_or(AutomationError::NotFound("schedule"))
    }

    async fn create(
        &mut self,
        mut definition: ScheduleDefinition,
        paused: bool,
        asker: Asker,
    ) -> Result<ScheduleInfo, AutomationError> {
        let (people, agents) = self.book.counts();
        let own = match &asker.by {
            Creator::Agent { session, .. } => self.live_of_agent(*session),
            Creator::Owner | Creator::Device { .. } => 0,
        };
        asker.may_create(own, people, agents)?;
        // A make that waited in the queue while its agent's session ended
        // would otherwise be kept after the end that deleted the others.
        if let Creator::Agent { session, .. } = &asker.by
            && !self.runner.agent_alive(*session)
        {
            return Err(AutomationError::Forbidden(
                "the agent's session has ended; its schedules went with it".to_owned(),
            ));
        }
        let now = self.clock.now();
        node_anchor(&asker, &mut definition);
        let first = settle(&self.zones, &mut definition, now)?;
        let state = if paused {
            ScheduleState::Paused { since: now }
        } else {
            ScheduleState::Active { next_due: first }
        };
        let info = ScheduleInfo {
            id: ScheduleId::random(),
            definition,
            state,
            runs: 0,
            last_due: None,
            last_run: None,
            defined_by: asker.by,
            created_at: now,
            updated_at: now,
            revision: 0,
        };
        self.store.put_schedule(info.clone(), None).await?;
        self.book.put(info.clone());
        Ok(info)
    }

    /// An agent session's schedules that have not ended (what its bound
    /// counts).
    fn live_of_agent(&self, session: SessionId) -> usize {
        self.book
            .of_agent(session)
            .into_iter()
            .filter_map(|id| self.book.get(id))
            .filter(|entry| !matches!(entry.info.state, ScheduleState::Ended { .. }))
            .count()
            .min(AGENT_SCHEDULES)
    }

    async fn update(
        &mut self,
        schedule: ScheduleId,
        revision: u32,
        mut definition: ScheduleDefinition,
        asker: &Asker,
    ) -> Result<ScheduleInfo, AutomationError> {
        let mut info = self.found(schedule)?.info.clone();
        asker.may(Act::Update, &info.defined_by)?;
        update_admitted(&info)?;
        if revision != info.revision {
            return Err(AutomationError::Conflict(format!(
                "the schedule was changed meanwhile (now revision {}); edit that one",
                info.revision
            )));
        }
        // A person taking an agent's schedule over makes it a person's,
        // which their bound counts.
        let to_people = matches!(info.defined_by, Creator::Agent { .. })
            && !matches!(asker.by, Creator::Agent { .. });
        if to_people && self.book.counts().0 >= PEOPLE_SCHEDULES {
            return Err(AutomationError::Conflict(
                "the node has as many schedules as it keeps (256); delete some first".to_owned(),
            ));
        }
        let now = self.clock.now();
        node_anchor(asker, &mut definition);
        let first = settle(&self.zones, &mut definition, now)?;
        let cadence_changed = definition.cadence != info.definition.cadence;
        let was_ended = matches!(info.state, ScheduleState::Ended { .. });
        info.state = match info.state {
            ScheduleState::Active { next_due } => {
                let next = if cadence_changed { first } else { next_due };
                advanced(&definition, Some(next), now)
            }
            other @ (ScheduleState::Paused { .. }
            | ScheduleState::Ended { .. }
            | ScheduleState::Unknown) => other,
        };
        // Updating limits must not rewrite an already completed reason/time.
        if !was_ended
            && let Some(max) = definition.max_runs
            && info.runs >= max.get()
        {
            info.state = ScheduleState::Ended {
                at: now,
                reason: EndReason::MaxRuns,
            };
        }
        info.definition = definition;
        info.defined_by = asker.by.clone();
        info.revision = info.revision.wrapping_add(1);
        info.updated_at = now;
        if ends_an_agents(&info) {
            self.forget(schedule).await;
            return Ok(info);
        }
        self.store.put_schedule(info.clone(), None).await?;
        self.book.put(info.clone());
        Ok(info)
    }

    async fn pause(
        &mut self,
        schedule: ScheduleId,
        asker: &Asker,
        pause: bool,
    ) -> Result<ScheduleInfo, AutomationError> {
        let mut info = self.found(schedule)?.info.clone();
        asker.may(
            if pause { Act::Pause } else { Act::Resume },
            &info.defined_by,
        )?;
        ended_refused(&info)?;
        let now = self.clock.now();
        let state = match (&info.state, pause) {
            (ScheduleState::Active { .. }, false) | (ScheduleState::Paused { .. }, true) => {
                return Ok(info);
            }
            (ScheduleState::Active { .. }, true) => ScheduleState::Paused { since: now },
            (ScheduleState::Paused { .. }, false) => {
                let next = Due::of(&info.definition.cadence, info.created_at, &self.zones)
                    .map_err(|error| AutomationError::Invalid(error.to_string()))
                    .map(|due| due.next_after(now))?;
                advanced(&info.definition, next, now)
            }
            (ScheduleState::Ended { .. } | ScheduleState::Unknown, _) => {
                return Err(AutomationError::Conflict(
                    "the schedule has ended".to_owned(),
                ));
            }
        };
        info.state = state;
        info.revision = info.revision.wrapping_add(1);
        info.updated_at = now;
        // Resumed past its end: an agent's goes, as at any other end.
        if ends_an_agents(&info) {
            self.forget(schedule).await;
            return Ok(info);
        }
        self.store.put_schedule(info.clone(), None).await?;
        self.book.put(info.clone());
        Ok(info)
    }

    async fn run_now(
        &mut self,
        schedule: ScheduleId,
        asker: Asker,
        reply: oneshot::Sender<Result<ScheduleRun, AutomationError>>,
    ) {
        let entry = match self.found(schedule) {
            Ok(entry) => entry.clone(),
            Err(error) => {
                let _gone = reply.send(Err(error));
                return;
            }
        };
        let refused = asker
            .may(Act::RunNow, &entry.info.defined_by)
            .and_then(|()| ended_refused(&entry.info));
        if let Err(error) = refused {
            let _gone = reply.send(Err(error));
            return;
        }
        if entry.making {
            let _gone = reply.send(Err(AutomationError::Conflict(
                "a run of this schedule is being made".to_owned(),
            )));
            return;
        }
        let now = self.clock.now();
        let trigger = RunTrigger::Asked { by: asker.by };
        self.start_run(entry.info, now, trigger, Some(reply)).await;
    }
}
