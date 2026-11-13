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

use std::time::Duration;

use vessel_proto::automation::{
    AfterRun, EndReason, RunEnd, RunError, RunOutcome, ScheduleRun, ScheduleTarget, SkipReason,
};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{
    ContentBlock, Creator, Delivered, Delivery, DisplayStatus, ErrorCode, EventSeq, Mark, Method,
    SessionId, SessionMode, SessionSpec, SessionStatus, SessionTitle, WireError,
};

use super::{NodeService, wire};
use crate::automation::schedules::{FollowSender, Followed, Job, Made};
use crate::automation::turns::{Signal, signal};
use crate::error::ServiceError;
use crate::principal::Principal;

/// Records a follow reads at a time.
const FOLLOW_PAGE: u32 = 200;
/// How long one read of a followed session waits for a record.
const FOLLOW_WAIT: Duration = Duration::from_secs(60);

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
            Err(error) => Made::outcome(failed(&wire(&error), Some(info.id))),
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

    /// Follow a `started` run's session to the end of its first turn (or
    /// its own end), sending each change of outcome; then, under
    /// `archive`, archive it if nothing waits in its queue.
    pub(crate) async fn follow_run(&self, job: Job, session: SessionId, follow: FollowSender) {
        let after = match &job.definition.target {
            ScheduleTarget::NewSession { after, .. } => *after,
            ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => AfterRun::Keep,
        };
        let mut from = EventSeq::ZERO;
        let mut current = RunOutcome::Started { session };
        let end = loop {
            let page = match self
                .sessions
                .read_waiting(session, from, FOLLOW_PAGE, FOLLOW_WAIT)
                .await
            {
                Ok(page) => page,
                Err(error) => {
                    tracing::debug!(%session, %error, "a followed session cannot be read");
                    break RunEnd::Session {
                        status: SessionStatus::Interrupted,
                    };
                }
            };
            let mut ended = None;
            for record in &page.records {
                match signal(&record.event) {
                    Some(Signal::Asks(_)) => {
                        current = RunOutcome::Waiting { session };
                        send(&follow, &job, current.clone()).await;
                    }
                    Some(Signal::Answered) if matches!(current, RunOutcome::Waiting { .. }) => {
                        current = RunOutcome::Started { session };
                        send(&follow, &job, current.clone()).await;
                    }
                    Some(Signal::Ended(end)) => {
                        ended = Some(end);
                        break;
                    }
                    Some(Signal::Started | Signal::Reply(_) | Signal::Answered) | None => {}
                }
            }
            if let Some(end) = ended {
                break end;
            }
            from = page.next;
        };
        // A turn that failed leaves its session live as one that ended
        // does; either is archived, or each run would leave one more.
        let turned = matches!(end, RunEnd::Turn { .. } | RunEnd::Failed { .. });
        send(&follow, &job, RunOutcome::Finished { session, end }).await;
        if turned && after == AfterRun::Archive {
            self.archive_if_idle(session).await;
        }
    }

    /// Archive a run's session (ADR 0026: it stops, its log stays) unless
    /// something waits in its queue, which someone else put there.
    async fn archive_if_idle(&self, session: SessionId) {
        let idle = self
            .sessions
            .conversation(session)
            .await
            .is_ok_and(|conversation| conversation.queue.entries.is_empty());
        if !idle {
            return;
        }
        if let Err(error) = self.sessions.mark(session, Mark::Archive).await {
            tracing::warn!(%session, %error, "cannot archive a schedule's session");
        }
    }

    /// At the node's start: an unfinished run of the last life of the node
    /// (ADR 0033). A run being made was interrupted; a followed session
    /// ended with that node.
    pub(crate) async fn settle_run(&self, run: &ScheduleRun) -> Option<RunOutcome> {
        match &run.outcome {
            RunOutcome::Making => Some(RunOutcome::Interrupted),
            RunOutcome::Started { session } | RunOutcome::Waiting { session } => {
                let status = match self.sessions.get(*session).await {
                    Ok(info) if info.status.is_live() => return None,
                    Ok(info) => info.status,
                    Err(_gone) => SessionStatus::Interrupted,
                };
                Some(RunOutcome::Finished {
                    session: *session,
                    end: RunEnd::Session { status },
                })
            }
            RunOutcome::Finished { .. }
            | RunOutcome::Prompted { .. }
            | RunOutcome::Skipped { .. }
            | RunOutcome::Failed { .. }
            | RunOutcome::Interrupted
            | RunOutcome::Unknown => None,
        }
    }
}

/// Tell the scheduler a followed run's outcome changed.
async fn send(follow: &FollowSender, job: &Job, outcome: RunOutcome) {
    let run = ScheduleRun {
        outcome,
        ..job.run.clone()
    };
    let _gone = follow
        .send(Followed {
            schedule: job.schedule,
            run,
        })
        .await;
}
