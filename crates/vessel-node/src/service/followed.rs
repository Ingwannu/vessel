//! A schedule's run after it was made (protocol 11, ADR 0033): its new
//! session followed to the end of its first turn and put away, and, at the
//! node's start, what an unfinished run of its last life came to. Making
//! the run is `scheduled`; this decides nothing about time.

use std::time::Duration;

use vessel_proto::automation::{AfterRun, RunArchive, RunEnd, RunOutcome, ScheduleRun, ScheduleTarget};
use vessel_proto::{EventSeq, SessionId, SessionStatus};
use vessel_session::IdleStop;

use super::NodeService;
use crate::automation::schedules::{FollowSender, Followed, Job};
use crate::automation::turns::{Signal, signal};

/// Records a follow reads at a time.
const FOLLOW_PAGE: u32 = 200;
/// How long one read of a followed session waits for a record.
const FOLLOW_WAIT: Duration = Duration::from_secs(60);

impl NodeService {
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
        let outcome = RunOutcome::Finished { session, end };
        if job.definition.target.isolation().is_some() {
            self.finish_isolated_run(&job, outcome, &follow).await;
            return;
        }
        send(&follow, &job, outcome).await;
        if turned && after == AfterRun::Archive {
            self.archive_if_idle(session).await;
        }
    }

    async fn finish_isolated_run(&self, job: &Job, outcome: RunOutcome, follow: &FollowSender) {
        let store = self.sessions.store();
        let binding = match store.schedule_run_binding(job.schedule, job.run.number).await {
            Ok(Some(binding)) => binding,
            Ok(None) => {
                send(follow, job, outcome).await;
                return;
            }
            Err(error) => {
                tracing::warn!(schedule = %job.schedule, %error, "cannot read the isolated run's binding");
                send(follow, job, outcome).await;
                return;
            }
        };
        let cleanup = self.dispose_isolation(binding).await;
        let archive = match &cleanup {
            Ok(cleanup) => cleanup.disposition(),
            Err(error) => {
                tracing::warn!(workspace = %binding.workspace, %error, "isolated workspace cleanup is unconfirmed");
                Some(RunArchive::Unconfirmed)
            }
        };
        let (committed, received) = tokio::sync::oneshot::channel();
        let run = ScheduleRun { outcome, archive, ..job.run.clone() };
        if follow.send(Followed { schedule: job.schedule, run, committed: Some(committed) }).await.is_ok()
            && received.await == Ok(true)
            && let Ok(cleanup) = cleanup
            && let Err(error) = self.settle_isolation(binding, cleanup).await
        {
            tracing::warn!(workspace = %binding.workspace, %error, "cannot release an isolated run's settled binding");
        }
    }

    /// Archive a run's session (ADR 0026: it stops, its log stays) unless
    /// something waits in its queue, which someone else put there; by the
    /// service's one path, so its terminals and services end and its pull
    /// request is no longer watched, as a person's archive does.
    async fn archive_if_idle(&self, session: SessionId) {
        match self.sessions.archive_idle(session).await {
            Ok(IdleStop::Stopped) => self.release_archived_session(session).await,
            Ok(IdleStop::Busy) => {}
            Err(error) => tracing::warn!(%session, %error, "cannot archive a schedule's session"),
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
            committed: None,
        })
        .await;
}
