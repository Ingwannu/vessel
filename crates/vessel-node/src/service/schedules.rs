//! Protocol 11's schedule calls (ADR 0033): who asks, what the target
//! names checked as `session.create` and `session.prompt` would check it,
//! then the scheduler (`automation::schedules`). Routing and those checks
//! only.

use vessel_proto::automation::{
    CreateSchedule, DeleteSchedule, GetSchedule, ListRuns, ListSchedules, PauseSchedule,
    ResumeSchedule, RunList, RunSchedule, ScheduleInfo, ScheduleList, ScheduleRun, ScheduleTarget,
    UpdateSchedule,
};
use vessel_proto::{Ack, ErrorCode, OriginBy, SessionMode, SessionSpec, WireError};

use super::{Connection, NodeService, wire};
use crate::automation::schedules::rules::{Asker, Via};
use crate::principal::Principal;
use crate::profiles::launch::creator;

/// Who asks, as the scheduler's rules see them.
pub(super) fn asker(principal: &Principal) -> Asker {
    let via = match principal {
        Principal::Owner | Principal::Device { .. } => Via::Person,
        Principal::Agent { .. } => Via::Agent,
        Principal::Delegated { origin, .. } => Via::Delegated {
            agent: matches!(origin.by, OriginBy::Agent { .. }),
        },
    };
    Asker {
        by: creator(principal),
        via,
    }
}

/// A session's spec kept with a schedule or a saved prompt, encoded, at
/// most: the bound of the prompt kept beside it.
const SPEC_MAX: usize = 16 * 1024;

fn invalid(message: &str) -> WireError {
    WireError::new(ErrorCode::InvalidRequest, message)
}

impl NodeService {
    pub(crate) async fn list_schedules(
        &self,
        _: &Connection,
        _: ListSchedules,
    ) -> Result<ScheduleList, WireError> {
        self.schedules
            .list()
            .await
            .map(|schedules| ScheduleList { schedules })
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn get_schedule(
        &self,
        _: &Connection,
        request: GetSchedule,
    ) -> Result<ScheduleInfo, WireError> {
        self.schedules
            .get(request.schedule)
            .await
            .map_err(|error| error.to_wire())
    }

    /// `schedule.create`: the target checked as a run would meet it, the
    /// time by the scheduler.
    pub(crate) async fn create_schedule(
        &self,
        connection: &Connection,
        request: CreateSchedule,
    ) -> Result<ScheduleInfo, WireError> {
        crate::label_ingress::target(&request.definition.target)?;
        let asker = asker(connection.principal());
        if !matches!(asker.via, Via::Delegated { .. }) {
            self.check_target(connection.principal(), &request.definition.target)
                .await?;
        }
        self.schedules
            .create(request.definition, request.paused, asker)
            .await
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn update_schedule(
        &self,
        connection: &Connection,
        request: UpdateSchedule,
    ) -> Result<ScheduleInfo, WireError> {
        crate::label_ingress::target(&request.definition.target)?;
        let asker = asker(connection.principal());
        if !matches!(asker.via, Via::Delegated { .. }) {
            self.check_target(connection.principal(), &request.definition.target)
                .await?;
        }
        self.schedules
            .update(
                request.schedule,
                request.revision,
                request.definition,
                asker,
            )
            .await
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn pause_schedule(
        &self,
        connection: &Connection,
        request: PauseSchedule,
    ) -> Result<ScheduleInfo, WireError> {
        self.schedules
            .pause(request.schedule, asker(connection.principal()))
            .await
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn resume_schedule(
        &self,
        connection: &Connection,
        request: ResumeSchedule,
    ) -> Result<ScheduleInfo, WireError> {
        self.schedules
            .resume(request.schedule, asker(connection.principal()))
            .await
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn run_schedule(
        &self,
        connection: &Connection,
        request: RunSchedule,
    ) -> Result<ScheduleRun, WireError> {
        self.schedules
            .run(request.schedule, asker(connection.principal()))
            .await
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn delete_schedule(
        &self,
        connection: &Connection,
        request: DeleteSchedule,
    ) -> Result<Ack, WireError> {
        self.schedules
            .delete(request.schedule, asker(connection.principal()))
            .await
            .map(|()| Ack)
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn list_runs(
        &self,
        _: &Connection,
        request: ListRuns,
    ) -> Result<RunList, WireError> {
        let _exists = self
            .schedules
            .get(request.schedule)
            .await
            .map_err(|error| error.to_wire())?;
        self.sessions
            .store()
            .runs(request.schedule)
            .await
            .map(|runs| RunList { runs })
            .map_err(|error| wire(&error.into()))
    }

    /// A spec a session will be started from later, without its caller
    /// there (a schedule's run, a saved prompt), checked for `principal`
    /// as `session.create` would check it now: structured, without
    /// `resume` (each start is a new conversation; `resume` would load the
    /// same one every time), its profile and place.
    pub(super) async fn check_new_spec(
        &self,
        principal: &Principal,
        spec: &SessionSpec,
    ) -> Result<(), WireError> {
        if spec.side_question_of.is_some() {
            return Err(crate::side_questions::SideError::Template.wire());
        }
        // Kept with a schedule or a saved prompt and sent in every list:
        // bounded as the prompt is, so 256 of them stay far under a frame
        // (a spec's folder, model and options are not bounded on their
        // own; the join's code review, finding 5).
        let size = serde_json::to_vec(spec).map_or(usize::MAX, |encoded| encoded.len());
        if size > SPEC_MAX {
            return Err(invalid(
                "a session's spec kept for later is at most 16 KiB encoded",
            ));
        }
        match &spec.mode {
            SessionMode::Terminal { .. } => {
                return Err(invalid(
                    "a session started later is structured: a terminal one cannot be sent its \
                     prompt",
                ));
            }
            SessionMode::Structured {
                resume: Some(_), ..
            } => {
                return Err(invalid(
                    "each start is a new conversation; `resume` would load the same one again \
                     every time",
                ));
            }
            SessionMode::Structured { resume: None, .. } => {}
        }
        let _launch = self
            .launcher
            .options(&self.profiles, spec, principal)
            .map_err(|error| wire(&error))?;
        // Resolved as a start would resolve it (ownership, the folder, a
        // worktree's setup), then let go: nothing starts here.
        let mut placed = spec.clone();
        let _placed = self.place(principal, &mut placed).await?;
        Ok(())
    }

    /// What a target names, checked for `principal` as a run would meet it
    /// (ADR 0033): a new session's spec as `session.create` takes it
    /// (structured, without `resume`, its profile and place), or a session
    /// that exists, is structured and is the caller's to prompt (an
    /// agent's own session: its heartbeat).
    async fn check_target(
        &self,
        principal: &Principal,
        target: &ScheduleTarget,
    ) -> Result<(), WireError> {
        match target {
            ScheduleTarget::NewSession { spec, .. } => self.check_new_spec(principal, spec).await,
            ScheduleTarget::Session { session } => {
                let info = self
                    .sessions
                    .get(*session)
                    .await
                    .map_err(|error| wire(&error))?;
                if info.spec.side_question_of.is_some() {
                    return Err(crate::side_questions::SideError::PromptOnly.wire());
                }
                if matches!(info.spec.mode, SessionMode::Terminal { .. }) {
                    return Err(WireError::new(
                        ErrorCode::Unsupported,
                        "a terminal session has no queue for a schedule's prompt",
                    ));
                }
                let own = principal
                    .agent()
                    .is_some_and(|(_, agent)| agent == *session);
                if own {
                    return Ok(());
                }
                self.check_owner(principal, *session).await
            }
            ScheduleTarget::Unknown => Err(invalid("a target this node does not know")),
        }
    }
}
