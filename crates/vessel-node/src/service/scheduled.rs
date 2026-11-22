//! A schedule's run made through the service (protocol 11, ADR 0033): as
//! its definer, rebuilt as they are now, through the checks a call of
//! theirs would meet. The scheduler (`automation::schedules`) decides when;
//! this decides nothing about time.
//!
//! [Decision Log]
//! - Purpose: a run is a `session.create` and a prompt, or a prompt through
//!   a session's queue, made later and unattended. It must never do what
//!   its definer could not do now, and it must say what it did.
//! - Constraint: the scope check and ADR 0013's ownership live in the
//!   guard, which only a connection's calls pass through; a run has no
//!   connection.
//! - Choice: here, beside the calls, the same checks in the same order:
//!   the method's scope (`Principal::permit`), ownership (`check_owner`),
//!   then the call's own path (`create_as`, `Sessions::prompt`). A refusal
//!   becomes the run's `failed` with the code the call would have answered.
//! - The one exception to ownership is a heartbeat: an agent's prompt to
//!   its own session, through its own queue.

use vessel_proto::automation::{
    AfterRun, EndReason, RunError, RunOutcome, ScheduleTarget, SkipReason,
};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{
    ContentBlock, Creator, Delivered, Delivery, DisplayStatus, ErrorCode, Method, SessionId,
    SessionMode, SessionSpec, SessionTitle, WireError,
};

use super::{NodeService, wire};
use crate::automation::schedules::{Job, Made};
use crate::error::ServiceError;
use crate::principal::Principal;

/// A run that failed with `error`.
fn failed(error: &WireError, session: Option<SessionId>) -> RunOutcome {
    RunOutcome::Failed {
        error: RunError::from_wire(error),
        session,
    }
}

fn skipped(reason: SkipReason) -> Made {
    Made::outcome(RunOutcome::Skipped { reason })
}

/// The schedule ends with this run.
fn ends(error: &WireError, reason: EndReason) -> Made {
    Made {
        outcome: failed(error, None),
        end: Some(reason),
    }
}

impl NodeService {
    /// Make `job`'s run.
    pub(crate) async fn make_run(&self, job: &Job) -> Made {
        let Some(principal) = self.definer(&job.defined_by).await else {
            let gone = WireError::new(
                ErrorCode::Forbidden,
                "the schedule's definer is gone (a revoked device, or an agent whose session \
                 ended)",
            );
            return ends(&gone, EndReason::CreatorGone);
        };
        match &job.definition.target {
            ScheduleTarget::NewSession { spec, after } => {
                self.run_new_session(job, &principal, spec, *after).await
            }
            ScheduleTarget::Session { session } => self.run_prompt(job, &principal, *session).await,
            ScheduleTarget::Unknown => Made::outcome(failed(
                &WireError::new(
                    ErrorCode::InvalidRequest,
                    "a target this node does not know",
                ),
                None,
            )),
        }
    }

    /// Who `defined_by` is now: `None` once they cannot act (ADR 0033).
    async fn definer(&self, defined_by: &Creator) -> Option<Principal> {
        match defined_by {
            Creator::Owner => Some(Principal::Owner),
            Creator::Device { id } => match self.devices.scopes_now(*id).await {
                Ok(scopes) => scopes.map(|scopes| Principal::Device { id: *id, scopes }),
                Err(error) => {
                    // Unknown is not gone: the run fails, the schedule stays.
                    tracing::warn!(device = %id, %error, "cannot read a schedule's device");
                    Some(Principal::Device {
                        id: *id,
                        scopes: Vec::new(),
                    })
                }
            },
            Creator::Agent { node, session } => {
                if *node != self.lifecycle.info().id {
                    return None;
                }
                self.agents.scopes(*session).map(|scopes| Principal::Agent {
                    node: *node,
                    session: *session,
                    scopes,
                })
            }
        }
    }

    async fn run_new_session(
        &self,
        job: &Job,
        principal: &Principal,
        spec: &SessionSpec,
        after: AfterRun,
    ) -> Made {
        if let Some(previous) = self.previous_session(job, after).await {
            tracing::debug!(schedule = %job.schedule, session = %previous, "previous run still running");
            return skipped(SkipReason::PreviousRunning);
        }
        if let Some(gone) = self.place_gone(spec).await {
            return ends(&gone, EndReason::TargetGone);
        }
        if let Err(error) = principal.permit(Method::CreateSession) {
            return Made::outcome(failed(&error, None));
        }
        let mut spec = spec.clone();
        if spec.title.is_none() {
            spec.title = job
                .definition
                .name
                .as_ref()
                .and_then(|name| name.as_str().parse::<SessionTitle>().ok());
        }
        let notice = format!(
            "vessel: started by schedule \"{}\" (run {})",
            job.name,
            job.run.number.get()
        );
        let info = match self.create_as(principal, spec, None, Some(notice)).await {
            Ok(info) => info,
            Err(error) => return Made::outcome(failed(&error, None)),
        };
        let content = vec![ContentBlock::Text {
            text: job.definition.prompt.as_str().to_owned(),
        }];
        let by = crate::profiles::launch::creator(principal);
        match self
            .sessions
            .prompt(info.id, content, Delivery::Queue, by)
            .await
        {
            Ok(_delivered) => Made::outcome(RunOutcome::Started { session: info.id }),
            Err(error) => {
                // Started but never prompted: nothing follows it, so it is
                // put away now, or each such run would leave one more live
                // (the join's code review, finding 11).
                if let Err(put) = self.put_away(info.id).await {
                    tracing::warn!(session = %info.id, error = %put, "cannot put away a schedule's session that got no prompt");
                }
                Made::outcome(failed(&wire(&error), Some(info.id)))
            }
        }
    }

    /// The previous run's session, when it still keeps this one from
    /// starting: live and in its first turn (`archive`), or live at all
    /// (`keep`).
    async fn previous_session(&self, job: &Job, after: AfterRun) -> Option<SessionId> {
        let session = match job.previous.as_ref().map(|run| &run.outcome)? {
            RunOutcome::Started { session } | RunOutcome::Waiting { session } => *session,
            RunOutcome::Finished { session, .. } if after == AfterRun::Keep => *session,
            RunOutcome::Finished { .. }
            | RunOutcome::Making
            | RunOutcome::Prompted { .. }
            | RunOutcome::Skipped { .. }
            | RunOutcome::Failed { .. }
            | RunOutcome::Interrupted
            | RunOutcome::Unknown => return None,
        };
        let live = self
            .sessions
            .get(session)
            .await
            .is_ok_and(|info| info.status.is_live());
        live.then_some(session)
    }

    /// The project or worktree a spec names, if it was removed.
    async fn place_gone(&self, spec: &SessionSpec) -> Option<WireError> {
        match spec.place.as_deref()? {
            SessionPlace::Project { project, .. } => self
                .workspaces
                .get_project(*project, None, false)
                .await
                .err()
                .map(|error| super::projects::wire(&error)),
            SessionPlace::Worktree { worktree } => self
                .workspaces
                .worktree(*worktree)
                .err()
                .map(|error| super::projects::wire(&error)),
        }
    }

    async fn run_prompt(&self, job: &Job, principal: &Principal, session: SessionId) -> Made {
        let _operation = match self
            .recovery
            .before_operation(session, crate::recovery::Intervention::Wait)
            .await
        {
            Ok(operation) => operation,
            Err(error) => return Made::outcome(failed(&error, None)),
        };
        if let Err(error) = principal.permit(Method::PromptSession) {
            return Made::outcome(failed(&error, None));
        }
        let info = match self.sessions.get(session).await {
            Ok(info) => info,
            // Only a session the node no longer has ends the schedule; a
            // store that cannot be read now fails this run alone.
            Err(error @ ServiceError::NotFound(_)) => {
                return ends(&wire(&error), EndReason::TargetGone);
            }
            Err(error) => return Made::outcome(failed(&wire(&error), None)),
        };
        let own = principal.agent().is_some_and(|(_, agent)| agent == session);
        if !own && let Err(error) = self.check_owner(principal, session).await {
            return Made::outcome(failed(&error, None));
        }
        if matches!(info.spec.mode, SessionMode::Terminal { .. }) {
            let error = WireError::new(
                ErrorCode::Unsupported,
                "a terminal session has no queue to send a prompt through",
            );
            return Made::outcome(failed(&error, None));
        }
        if !info.status.is_live() {
            let error = WireError::new(
                ErrorCode::Conflict,
                "the session has ended; start it again for the schedule to reach it",
            );
            return Made::outcome(failed(&error, None));
        }
        if self.still_waiting(job, session).await {
            return skipped(SkipReason::PreviousWaiting);
        }
        // Nothing automatic starts a session a person stopped (ADR 0024's
        // rule, kept for prompts that come after the stop).
        if info
            .board
            .as_ref()
            .is_some_and(|board| board.display_status == DisplayStatus::Interrupted)
        {
            return skipped(SkipReason::Stopped);
        }
        let content = vec![ContentBlock::Text {
            text: job.definition.prompt.as_str().to_owned(),
        }];
        let by = crate::profiles::launch::creator(principal);
        match self
            .sessions
            .prompt(session, content, Delivery::Queue, by)
            .await
        {
            Ok(delivered) => Made::outcome(RunOutcome::Prompted { session, delivered }),
            Err(error) => Made::outcome(failed(&wire(&error), None)),
        }
    }

    /// Whether the previous run's prompt still waits in `session`'s queue.
    async fn still_waiting(&self, job: &Job, session: SessionId) -> bool {
        let Some(RunOutcome::Prompted {
            session: previous,
            delivered: Delivered::Queued { entry },
        }) = job.previous.as_ref().map(|run| &run.outcome)
        else {
            return false;
        };
        if *previous != session {
            return false;
        }
        self.sessions
            .conversation(session)
            .await
            .is_ok_and(|conversation| {
                conversation
                    .queue
                    .entries
                    .iter()
                    .any(|waiting| waiting.id == *entry)
            })
    }
}
