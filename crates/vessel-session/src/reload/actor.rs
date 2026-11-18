//! The actor's reload step (TRACKER 803; order and failures in the
//! parent's Decision Log): one order, carried out between two others, so
//! nothing reaches the agent while it is replaced.

use tokio::sync::oneshot;
use vessel_proto::parking::ParkReason;
use vessel_proto::{HarnessSessionId, SessionEvent};

use super::rehydrate::{self, Gather, Outcome};
use super::{NotReloadable, UserPrompts};
use crate::error::SessionError;
use crate::moving::{Launch, MoveError};
use crate::parking::Route;
use crate::structured::StructuredActor;

/// A reload, from the runtime (`SessionRuntime::reload`).
#[derive(Debug)]
pub(crate) struct ReloadOrder {
    /// The fresh launch: the session's driver, command and setup, with its
    /// conversation and choices filled in by the actor.
    pub(crate) launch: Launch,
    /// The log's user prompts before this run.
    pub(crate) before: UserPrompts,
    /// Lines for the log once the agent is ready again.
    pub(crate) notices: Vec<String>,
    pub(crate) reply: oneshot::Sender<Result<(), SessionError>>,
}

impl StructuredActor {
    /// Carry out a reload and answer it.
    pub(crate) async fn reload(&mut self, order: ReloadOrder) {
        let ReloadOrder {
            mut launch,
            before,
            notices,
            reply,
        } = order;
        // Whether the agent may ask is the session's (protocol 10), as for
        // a move.
        launch.setup.questions = self.launch.setup.questions;
        let reloaded = self.try_reload(launch, &before).await;
        match &reloaded {
            Ok(()) => self.life.notices(&mut self.agent, notices).await,
            Err(error) => tracing::info!(%error, "the session's agent was not reloaded"),
        }
        let _gone = reply.send(reloaded);
    }

    async fn try_reload(
        &mut self,
        mut launch: Launch,
        before: &UserPrompts,
    ) -> Result<(), SessionError> {
        if !self.ready || self.move_state.active {
            return Err(NotReloadable::NotReady.into());
        }
        let Some(conversation) = self.parking.harness_session.clone() else {
            return Err(NotReloadable::NoConversation.into());
        };
        if !self.desk.abilities().resume {
            return Err(NotReloadable::AgentCannot.into());
        }
        launch.setup.resume = Some(conversation.clone());
        launch.setup.options.clone_from(&self.parking.choices);
        let parked = self.parking.parked.is_some();
        let _held = self.route.send_replace(Route::Moving);
        self.move_state.active = true;
        let stopped = if parked {
            Ok(())
        } else {
            self.stop_running().await
        };
        let started = match stopped {
            Ok(()) => {
                self.move_state.gather = Some(Gather::new());
                self.start_agent(&launch)
                    .await
                    .map_err(|_stopped| SessionError::Ended)
            }
            Err(refusal) => Err(refusal),
        };
        self.move_state.active = false;
        let gather = self.move_state.gather.take();
        match started {
            Ok(Ok(())) if self.parking.harness_session.as_ref() == Some(&conversation) => {
                // The sign-in it runs under now, as a move records it.
                self.life.out.set_limit_association(launch.limits.clone());
                self.launch = launch;
                self.parking.parked = None;
                self.disconnect = None;
                self.failed_before_kill = false;
                let _live = self.route.send_replace(Route::Live(self.input.clone()));
                self.reloaded(before, gather.unwrap_or_else(Gather::new), conversation)
                    .await;
                self.parking.touch();
                Ok(())
            }
            Ok(Ok(())) => {
                // Ready on another conversation: not the session's.
                self.parking.harness_session = Some(conversation);
                self.stop_agent()
                    .await
                    .map_err(|_stopped| SessionError::Ended)?;
                Err(self
                    .failed(parked, "the agent loaded another conversation".to_owned())
                    .await)
            }
            Ok(Err(failure)) => Err(self.failed(parked, failure.to_string()).await),
            Err(SessionError::Ended) => Err(SessionError::Ended),
            // Refused before anything new started: the agent that ran runs.
            Err(refusal) => {
                let _live = self.route.send_replace(if parked {
                    Route::Parked
                } else {
                    Route::Live(self.input.clone())
                });
                Err(refusal)
            }
        }
    }

    /// Cancel a running turn, waiting for its end as a forced move does,
    /// then stop the agent. A turn that does not end is refused and kept.
    async fn stop_running(&mut self) -> Result<(), SessionError> {
        let input = self.input.clone();
        let running = self
            .pump(async move { input.turn_running().await })
            .await
            .map_err(|_stopped| SessionError::Ended)?;
        if running.unwrap_or(false) {
            self.move_state.turn_over = false;
            let input = self.input.clone();
            let _cancelled = self
                .pump(async move { input.cancel_turn().await })
                .await
                .map_err(|_stopped| SessionError::Ended)?;
            let over = self
                .until_turn_over()
                .await
                .map_err(|_stopped| SessionError::Ended)?;
            if !over {
                return Err(NotReloadable::TurnDidNotEnd.into());
            }
        }
        self.stop_agent().await.map_err(|error| match error {
            MoveError::Signal { .. } => NotReloadable::CannotStop.into(),
            MoveError::Stopped => SessionError::Ended,
            other @ (MoveError::Terminal
            | MoveError::NotReady
            | MoveError::TurnInProgress
            | MoveError::Files(_)
            | MoveError::Link(_)
            | MoveError::Restart(_)
            | MoveError::Task(_)) => other.into(),
        })
    }

    /// The new agent did not become the session's. A parked session stays
    /// parked, saying why, as after a failed wake; a running one ends
    /// `failed` once the loop sees no agent, its log kept for
    /// `session.resume`.
    async fn failed(&mut self, parked: bool, message: String) -> SessionError {
        tracing::info!(%message, "the reloaded agent did not start");
        self.outputs_open = false;
        if parked {
            self.enter_parked(ParkReason::wake_failed(&message)).await;
        } else {
            self.unrecoverable = Some(format!(
                "the agent did not start again after a reload ({message}); resume the session \
                 to go on"
            ));
            let _gone = self.route.send_replace(Route::Moving);
        }
        SessionError::ReloadFailed(message)
    }

    /// Publish `reloaded`, then what the agent's history adds (`rehydrate`),
    /// then the line saying why nothing was added, if that is the case.
    pub(crate) async fn reloaded(
        &mut self,
        before: &UserPrompts,
        gather: Gather,
        conversation: HarnessSessionId,
    ) {
        let log = before.clone().then(self.life.out.prompts());
        let (records, line) = match rehydrate::outcome(&log, gather) {
            Outcome::Current => (Vec::new(), None),
            Outcome::Append { records, more } => (records, more.then_some(rehydrate::MORE)),
            Outcome::Nothing(line) => (Vec::new(), Some(line)),
        };
        let appended = u32::try_from(records.len()).unwrap_or(u32::MAX);
        self.life
            .publish(
                &mut self.agent,
                SessionEvent::Reloaded {
                    harness_session: conversation,
                    appended,
                },
            )
            .await;
        for event in records {
            self.life
                .publish(&mut self.agent, SessionEvent::Agent { event })
                .await;
        }
        if let Some(line) = line {
            self.life
                .notices(&mut self.agent, vec![line.to_owned()])
                .await;
        }
    }
}
