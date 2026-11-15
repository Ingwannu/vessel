//! Rewinding a structured session to a turn's end (protocol 14, ADR 0052):
//! its agent's conversation is forked at the point and the session goes on
//! with the fork; what came after stays in the log.
//!
//! Owns: the actor's rewind, from holding input to the `rewound` record.
//! Must not: find the point (the node reads it from the log), or count
//! what was left behind (the node, from the stored records).
//!
//! [Decision Log]
//! - Purpose: a person takes a conversation back to an earlier turn.
//! - Constraint: over ACP no adapter rewinds in place; both installed ones
//!   fork at a point, and a fork at a point is checked by the agent's own
//!   driver (`vessel-agent`: the fork is loaded and its replay must end at
//!   the point).
//! - Choice: a move's machinery (input held, the agent stopped, another
//!   started, the replay dropped) with a fork for the start. On any failure
//!   the old conversation starts again, with the session's current
//!   choices, and the call is answered with why; the log then has no
//!   `rewound` record. A parked session is woken first, as a move wakes it.
//! - The `rewound` record carries the new conversation id: a restarted
//!   agent's `ready` is not logged, and a later move, resume or export
//!   reads the id from the record (the store's projection, the info).

use tokio::sync::oneshot;
use vessel_agent::{AgentError, Disconnect, ForkFrom, LaunchRefusal};
use vessel_proto::{EventSeq, SessionEvent, TurnPoint};

use crate::error::{RewindRefusal, SessionError};
use crate::moving::Restart;
use crate::parking::Route;
use crate::structured::StructuredActor;

/// A rewind a handle asked for.
#[derive(Debug)]
pub(crate) struct RewindOrder {
    /// Where to: the turn's last message, in the conversation it is in.
    pub(crate) point: TurnPoint,
    /// The `turn_ended` record that carries it.
    pub(crate) after: EventSeq,
    /// Tool calls left behind that changed files (counted by the node).
    pub(crate) files_changed: u32,
    pub(crate) reply: oneshot::Sender<Result<(), SessionError>>,
}

impl StructuredActor {
    /// Why the session cannot be rewound now, if it cannot.
    fn rewind_refusal(&self) -> Option<RewindRefusal> {
        if !self.ready || self.move_state.active {
            return Some(RewindRefusal::NotReady);
        }
        if self.desk.running() {
            return Some(RewindRefusal::TurnRunning);
        }
        if self.life.out.waiting_for_answers() {
            return Some(RewindRefusal::Waiting);
        }
        if !self.desk.queue.is_empty() && !self.desk.queue.is_held() {
            return Some(RewindRefusal::QueueMoving);
        }
        None
    }

    /// Carry out a rewind and answer it.
    pub(crate) async fn rewind(&mut self, order: RewindOrder) {
        let RewindOrder {
            point,
            after,
            files_changed,
            reply,
        } = order;
        let rewound = self.try_rewind(point, after, files_changed).await;
        if let Err(error) = &rewound {
            tracing::info!(%error, "the session was not rewound");
        }
        let _gone = reply.send(rewound);
    }

    async fn try_rewind(
        &mut self,
        point: TurnPoint,
        after: EventSeq,
        files_changed: u32,
    ) -> Result<(), SessionError> {
        if self.parking.parked.is_some() {
            self.wake().await?;
        }
        if let Some(refusal) = self.rewind_refusal() {
            return Err(refusal.into());
        }
        let _held = self.route.send_replace(Route::Moving);
        self.move_state.active = true;
        let rewound = self.fork_in_place(point, after, files_changed).await;
        self.move_state.active = false;
        if !matches!(rewound, Err(SessionError::Ended)) {
            let _live = self.route.send_replace(Route::Live(self.input.clone()));
        }
        rewound
    }

    /// Stop the agent, start it on a fork of `point`'s conversation, and
    /// say so; on a failure start the old conversation again.
    async fn fork_in_place(
        &mut self,
        point: TurnPoint,
        after: EventSeq,
        files_changed: u32,
    ) -> Result<(), SessionError> {
        let old = self.parking.harness_session.clone();
        self.stop_agent()
            .await
            .map_err(|_stopped| SessionError::Ended)?;
        let mut launch = self.launch.clone();
        launch.setup.fork = Some(Box::new(ForkFrom {
            session: point.conversation,
            point: Some(point.message),
        }));
        launch.setup.resume = None;
        launch.setup.options.clone_from(&self.parking.choices);
        let started = self
            .start_agent(&launch)
            .await
            .map_err(|_stopped| SessionError::Ended)?;
        match started {
            Ok(()) => {
                // The fork's `ready`, dropped with its replay, set it.
                let Some(conversation) = self.parking.harness_session.clone() else {
                    return Err(SessionError::RewindFailed(
                        "the agent named no conversation".to_owned(),
                    ));
                };
                launch.setup.fork = None;
                launch.setup.resume = Some(conversation.clone());
                self.launch = launch;
                self.disconnect = None;
                self.failed_before_kill = false;
                self.life
                    .publish(
                        &mut self.agent,
                        SessionEvent::Rewound {
                            after,
                            harness_session: conversation,
                            files_changed,
                        },
                    )
                    .await;
                Ok(())
            }
            Err(failure) => {
                // The fork's agent is gone (a failed start is killed): the
                // old conversation again, with the choices as they stand.
                let mut previous = self.launch.clone();
                previous.setup.fork = None;
                previous.setup.resume.clone_from(&old);
                previous.setup.options.clone_from(&self.parking.choices);
                match self
                    .start_agent(&previous)
                    .await
                    .map_err(|_stopped| SessionError::Ended)?
                {
                    Ok(()) => self.parking.harness_session = old,
                    Err(again) => {
                        // No agent runs now, and none will: the session
                        // ends `failed` (`unrecoverable`) as soon as the
                        // loop sees the agent gone, its conversation id the
                        // one it had (no `ready` or `rewound` was logged),
                        // so `session.resume` picks it up from before the
                        // rewind. Not parked: a wake would only try the
                        // same start again without anyone asking.
                        tracing::warn!(%again, "the old conversation did not start again either");
                        self.parking.harness_session = old;
                        self.outputs_open = false;
                        let message = format!(
                            "the rewind failed and the agent could not start the conversation \
                             again ({again}); resume the session to go on from before the rewind"
                        );
                        self.unrecoverable = Some(message.clone());
                        return Err(SessionError::RewindFailed(message));
                    }
                }
                Err(rewind_failed(failure))
            }
        }
    }
}

/// A rewind whose fork did not start, as the session answers it. A fork
/// the agent cannot make is the same answer whether its adapter refused it
/// in the handshake or a direct driver before spawning
/// (`LaunchRefusal::Fork`, ADR 0057): `ForkRefused`, `unsupported`.
fn rewind_failed(failure: Restart) -> SessionError {
    match failure {
        Restart::Handshake(Disconnect::Handshake {
            fork_refused: Some(refusal),
            ..
        })
        | Restart::Spawn(AgentError::Refused(LaunchRefusal::Fork(refusal))) => {
            SessionError::ForkRefused(refusal)
        }
        other @ (Restart::Admission(_)
        | Restart::Spawn(_)
        | Restart::Handshake(_)
        | Restart::Exited(_)
        | Restart::Woken(_)) => SessionError::RewindFailed(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use vessel_agent::{AgentError, Disconnect, Driver, ForkRefusal, LaunchRefusal};

    use super::rewind_failed;
    use crate::error::SessionError;
    use crate::moving::Restart;

    #[test]
    fn a_fork_refused_before_or_in_the_handshake_is_the_same_answer() {
        let refused_before = rewind_failed(Restart::Spawn(AgentError::Refused(
            LaunchRefusal::Fork(ForkRefusal::NotAtPoint),
        )));
        assert!(matches!(
            refused_before,
            SessionError::ForkRefused(ForkRefusal::NotAtPoint)
        ));
        let refused_in = rewind_failed(Restart::Handshake(Disconnect::Handshake {
            message: "no".to_owned(),
            refused: None,
            fork_refused: Some(ForkRefusal::NotAtPoint),
        }));
        assert!(matches!(
            refused_in,
            SessionError::ForkRefused(ForkRefusal::NotAtPoint)
        ));
        assert!(matches!(
            rewind_failed(Restart::Spawn(AgentError::NoDriver(Driver::ClaudeStream))),
            SessionError::RewindFailed(_)
        ));
    }
}
