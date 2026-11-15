//! Boot-only recovery through the same launch, grants and actor input as a person.
use super::{NodeService, wire};
mod attempt;
#[cfg(test)]
mod tests;
use crate::recovery::{Attempt, policy::Generation};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vessel_proto::{ErrorCode, EventRecord, SessionEvent, SessionId, SessionStatus, WireError};
use vessel_session::SessionHandle;
use vessel_store::RestartCandidate;

const ADMISSION: Duration = Duration::from_secs(120);
/// Driver error characters retained inside a node-authored outcome envelope.
const ERROR_DETAIL_CHARS: usize = 512;

mod startup;
pub(crate) use startup::start;

fn settled(result: Option<Result<(), tokio::task::JoinError>>) {
    if let Some(Err(error)) = result {
        tracing::error!(%error, "startup recovery owner failed");
    }
}
impl NodeService {
    async fn recover(
        &self,
        candidate: RestartCandidate,
        attempt: Attempt,
        stop: CancellationToken,
    ) {
        let deadline = Instant::now()
            .checked_add(ADMISSION)
            .unwrap_or_else(Instant::now);
        let generation = self.recovery.boot_generation;
        let mut run = Run::Unclaimed;
        let result = self
            .recover_turn(
                &Context {
                    candidate,
                    attempt: &attempt,
                    stop: &stop,
                    deadline,
                    generation,
                },
                &mut run,
            )
            .await;
        if let Err(error) = result {
            tracing::warn!(session = %candidate.session, code = ?error.code, "startup recovery did not complete");
            if !matches!(run, Run::Unclaimed) {
                self.recovery_failed(candidate.session, &run, &error).await;
            }
            if matches!(run, Run::NotLaunched) {
                self.agents.forget(candidate.session);
                self.models.retire(candidate.session);
            }
            // External mutations wait for this exact owner. Do not kill a run
            // that was replaced, or one whose prompt was already admitted.
            if let Run::Loading(handle) = &run
                && self
                    .sessions
                    .runtime_handle(candidate.session)
                    .is_some_and(|current| current.same_run(handle))
            {
                // This is cleanup of our own unprompted launch, not a
                // person's stop or a successful suppression acknowledgement.
                handle.kill();
                let _ended = handle.ended().await;
            }
        }
        if let Err(error) = self
            .sessions
            .store()
            .finish_restart(candidate.session)
            .await
        {
            tracing::error!(%error, "cannot release startup recovery retention protection");
        }
        drop(attempt);
    }

    async fn stage<F: Future>(
        &self,
        context: &Context<'_>,
        future: F,
    ) -> Result<F::Output, WireError> {
        let Context {
            attempt,
            stop,
            deadline,
            generation,
            ..
        } = *context;
        let mut changed = self.recovery.policy.watch();
        let mut future = std::pin::pin!(future);
        loop {
            // Preparation/read futures keep making progress through an Off
            // save. Pausing an opaque fresh-grant future here could retain
            // its device fence across that unrelated wait. Only the final
            // synchronous admission may dispatch; it retains its FnOnce on
            // the temporary barrier, drops authority, then waits for change.
            let _available = self.recovery.policy.check(generation)?;
            tokio::select! {
                biased;
                () = stop.cancelled() => return Err(WireError::new(ErrorCode::Unavailable, "the node stopped before startup recovery admission")),
                () = attempt.cancelled.cancelled() => return Err(WireError::new(ErrorCode::Conflict, "a person stopped startup recovery")),
                () = tokio::time::sleep_until(deadline) => return Err(WireError::new(ErrorCode::Unavailable, format!("startup recovery did not become ready within {} seconds; resume it manually", ADMISSION.as_secs()))),
                _changed = changed.changed() => {},
                outcome = &mut future => return Ok(outcome),
            }
        }
    }

    async fn recovery_failed(&self, id: SessionId, run: &Run, error: &WireError) {
        let outcome = if matches!(run, Run::PromptAdmitted(_)) {
            "startup recovery admitted the prompt but did not report success; it will not be retried"
        } else {
            "startup recovery did not send a continuation"
        };
        let line = format!(
            "vessel: {outcome} ({})",
            error
                .message
                .chars()
                .take(ERROR_DETAIL_CHARS)
                .collect::<String>()
        );
        if let Some(handle) = run.handle() {
            if let Err(error) = handle.recovery_notice(line).await {
                tracing::warn!(%error, "cannot record startup recovery failure in the actor");
            }
            return;
        }
        let record = match self.sessions.store().session(id).await {
            Ok(Some(info)) if !info.status.is_live() => EventRecord {
                seq: info.next_seq,
                at: Utc::now(),
                event: SessionEvent::Status {
                    status: SessionStatus::failed(line),
                },
            },
            Ok(_) => return,
            Err(error) => {
                tracing::error!(%error, "cannot read the failed startup recovery row");
                return;
            }
        };
        if let Err(error) = self
            .sessions
            .store()
            .append(id, vec![Arc::new(record)])
            .await
        {
            tracing::error!(%error, "cannot record startup recovery refusal");
        }
    }
}

/// Cleanup authority ends once a prompt is admitted, even if its reply is
/// lost. Keep the handle separately for an actor-ordered failure diagnostic.
enum Run {
    Unclaimed,
    NotLaunched,
    Loading(SessionHandle),
    PromptAdmitted(SessionHandle),
}
impl Run {
    fn handle(&self) -> Option<&SessionHandle> {
        match self {
            Self::Unclaimed | Self::NotLaunched => None,
            Self::Loading(handle) | Self::PromptAdmitted(handle) => Some(handle),
        }
    }
}

#[derive(Clone, Copy)]
struct Context<'a> {
    candidate: RestartCandidate,
    attempt: &'a Attempt,
    stop: &'a CancellationToken,
    deadline: Instant,
    generation: Generation,
}
