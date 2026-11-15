//! Parking a structured session's agent and waking it (protocol 12, ADR
//! 0038): the agent's process is stopped while the session stays live and
//! `running`, and started again with its conversation (`session/load`) by
//! whatever next needs it.
//!
//! Owns: the route handles send through ([`Route`]), the actor's idle
//! clock and its parked state ([`Parking`]), and the actor's park and wake.
//! Must not: decide who may park a session (the node's guard), or keep
//! anything on disk (a node restart interrupts a parked session as any
//! live one).
//!
//! [Decision Log]
//! - Purpose: an idle agent holds its process (hundreds of MiB for the
//!   real agents) for nothing; the references release it after 10 to 60
//!   minutes and bring it back on the next prompt.
//! - Constraint: the actor's loop ends when its agent exits; handles send
//!   some input straight to the agent; a move to another profile already
//!   stops one agent and starts another on the same conversation.
//! - Alternatives: a sweep over every session every so often (a loop that
//!   wakes for nothing); ending the session and starting it again with
//!   `session.resume` (the session would leave the running lane and its
//!   followers would see it end).
//! - Choice: each actor keeps its own deadline, armed once per idle
//!   period (activity only notes the time; the timer, when it fires,
//!   re-arms itself if something happened since), and reuses the move's
//!   stop and start. The route says whether the agent is live, moving or
//!   parked, so orders reach a parked session's actor and wake it, while
//!   what needs the live agent itself is answered from what was kept.
//! - Cost: a parked session keeps its actor, its record ring and its last
//!   options; its first prompt waits for the agent's handshake and replay.
//! - Who follows it does not matter (2026-11-14, ADR 0038's amendment):
//!   Paseo, the owner's reference, collects an idle runtime whether or not
//!   a client watches it (its eligibility is turns, permissions, queued
//!   deliveries and running children). A follower loses nothing: the
//!   session stays live, its subscription goes on through `parked` and
//!   `woken`, and its next prompt wakes the agent. The earlier rule (no
//!   live follower, agent-orchestrator's view lease) let any client that
//!   kept a subscription open, or a connection the node never saw close,
//!   hold every agent it had shown.

use std::pin::Pin;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::oneshot;
use tokio::time::{Instant, Sleep};
use vessel_agent::AgentInput;
use vessel_proto::parking::{ParkReason, Parked};
use vessel_proto::{HarnessSessionId, OptionChoice, SessionEvent};

use crate::error::{ParkRefusal, SessionError};
use crate::moving::Launch;
use crate::structured::StructuredActor;

/// Where a structured session's input goes now.
#[derive(Debug, Clone)]
pub(crate) enum Route {
    /// To its agent.
    Live(AgentInput),
    /// Nowhere while its agent is replaced (a move, a park): orders wait.
    Moving,
    /// Its agent is parked: orders go to the actor, which wakes it.
    Parked,
}

impl Route {
    /// The live agent's input.
    pub(crate) fn input(&self) -> Result<AgentInput, SessionError> {
        match self {
            Self::Live(input) => Ok(input.clone()),
            Self::Moving => Err(SessionError::Moving),
            Self::Parked => Err(SessionError::Parked),
        }
    }
}

/// An order to park or wake, from a handle.
#[derive(Debug)]
pub(crate) enum ParkOrder {
    Park(oneshot::Sender<Result<(), SessionError>>),
    Wake(oneshot::Sender<Result<(), SessionError>>),
}

/// The actor's parking state.
#[derive(Debug)]
pub(crate) struct Parking {
    /// How long it may stay idle; `None` never parks on its own.
    after: Option<Duration>,
    /// When something last happened in it.
    active_at: Instant,
    /// Fires once per idle period; boxed once, reset when it fires.
    timer: Pin<Box<Sleep>>,
    /// Parked since when, and why.
    pub(crate) parked: Option<Parked>,
    /// The agent's session id, for `session/load`.
    pub(crate) harness_session: Option<HarnessSessionId>,
    /// The agent's options as it last reported them, as choices: what a
    /// wake starts it with (its permissions mode above all).
    pub(crate) choices: Vec<OptionChoice>,
}

impl Parking {
    pub(crate) fn new(
        after: Option<Duration>,
        choices: Vec<OptionChoice>,
        harness_session: Option<HarnessSessionId>,
    ) -> Self {
        let now = Instant::now();
        Self {
            after,
            active_at: now,
            timer: Box::pin(tokio::time::sleep_until(deadline(now, after))),
            parked: None,
            harness_session,
            choices,
        }
    }

    /// Something happened: the idle period starts again. Only the time is
    /// noted; the timer moves when it fires.
    pub(crate) fn touch(&mut self) {
        self.active_at = Instant::now();
    }

    /// Whether the idle timer is worth waiting on.
    pub(crate) const fn armed(&self) -> bool {
        self.after.is_some() && self.parked.is_none()
    }

    /// The idle timer, for the actor's `select`.
    pub(crate) const fn timer(&mut self) -> &mut Pin<Box<Sleep>> {
        &mut self.timer
    }

    /// The timer fired: whether the session has been idle for the whole
    /// period. If not, the timer is moved to the end of the period that
    /// started at the last activity.
    pub(crate) fn idle_long_enough(&mut self) -> bool {
        let due = deadline(self.active_at, self.after);
        if due <= Instant::now() {
            return true;
        }
        self.timer.as_mut().reset(due);
        false
    }

    /// The node's setting changed: the timer is moved to the end of the
    /// period that started at the last activity, which may already be past
    /// (the session is then looked at at once). A park already under way
    /// finishes.
    pub(crate) fn set_after(&mut self, after: Option<Duration>) {
        self.after = after;
        let due = deadline(self.active_at, self.after);
        self.timer.as_mut().reset(due);
    }

    /// Start a full idle period from now (after a park was not possible).
    pub(crate) fn rearm(&mut self) {
        self.touch();
        let due = deadline(self.active_at, self.after);
        self.timer.as_mut().reset(due);
    }
}

/// When a period of `after` that started at `from` ends; far away when
/// parking is off (the timer is then never waited on).
fn deadline(from: Instant, after: Option<Duration>) -> Instant {
    let after = after.unwrap_or(Duration::from_secs(365 * 24 * 60 * 60));
    from.checked_add(after).unwrap_or(from)
}

impl StructuredActor {
    /// Why the session cannot be parked now, if it cannot.
    fn refusal(&self) -> Option<ParkRefusal> {
        if !self.desk.abilities().resume || self.parking.harness_session.is_none() {
            return Some(ParkRefusal::CannotResume);
        }
        if !self.ready || self.move_state.active {
            return Some(ParkRefusal::NotReady);
        }
        if self.desk.running() {
            return Some(ParkRefusal::TurnRunning);
        }
        if self.life.out.waiting_for_answers() {
            return Some(ParkRefusal::Waiting);
        }
        if !self.desk.queue.is_empty() && !self.desk.queue.is_held() {
            return Some(ParkRefusal::QueueMoving);
        }
        None
    }

    /// The idle timer fired: park if nothing happened for the whole period
    /// and nothing prevents it (a turn, an open question or permission, a
    /// moving queue). Followers do not: they follow on through the park.
    pub(crate) async fn idle(&mut self) {
        if !self.parking.idle_long_enough() {
            return;
        }
        if self.refusal().is_some() {
            self.parking.rearm();
            return;
        }
        if let Err(error) = self.park(ParkReason::Idle).await {
            tracing::debug!(%error, "the idle agent was not parked");
        }
    }

    /// Carry out a park or a wake a handle asked for, and answer it.
    pub(crate) async fn park_order(&mut self, order: ParkOrder) {
        self.parking.touch();
        match order {
            ParkOrder::Park(reply) => {
                let parked = if self.parking.parked.is_some() {
                    Ok(())
                } else if let Some(refusal) = self.refusal() {
                    Err(refusal.into())
                } else {
                    self.park(ParkReason::Asked).await
                };
                let _gone = reply.send(parked);
            }
            ParkOrder::Wake(reply) => {
                let woken = self.wake().await;
                let _gone = reply.send(woken);
            }
        }
    }

    /// Stop the agent, as a move stops the old one, and say so. Orders that
    /// arrive meanwhile wait in the actor's channel; what goes straight to
    /// the agent is held as during a move.
    async fn park(&mut self, reason: ParkReason) -> Result<(), SessionError> {
        let _live = self.route.send_replace(Route::Moving);
        self.move_state.active = true;
        let stopped = self.stop_agent().await;
        self.move_state.active = false;
        if stopped.is_err() {
            // Stopped (killed or the node stopping) while parking: the
            // agent got the stop, and the loop ends the session.
            return Err(SessionError::Ended);
        }
        self.life.out.running(None);
        self.enter_parked(reason).await;
        tracing::debug!("parked the session's agent");
        Ok(())
    }

    /// Record the parked state and publish it.
    async fn enter_parked(&mut self, reason: ParkReason) {
        self.parking.parked = Some(Parked {
            since: Utc::now(),
            reason: reason.clone(),
        });
        let _held = self.route.send_replace(Route::Parked);
        self.life
            .publish(&mut self.agent, SessionEvent::Parked { reason })
            .await;
    }

    /// Start a parked session's agent again with its conversation and its
    /// current choices; nothing when it is not parked. A failure leaves it
    /// parked, saying why.
    pub(crate) async fn wake(&mut self) -> Result<(), SessionError> {
        if self.parking.parked.is_none() {
            return Ok(());
        }
        let mut launch: Launch = self.launch.clone();
        launch
            .setup
            .resume
            .clone_from(&self.parking.harness_session);
        launch.setup.options.clone_from(&self.parking.choices);
        let _held = self.route.send_replace(Route::Moving);
        self.move_state.active = true;
        let started = self.start_agent(&launch).await;
        self.move_state.active = false;
        match started {
            Ok(Ok(())) => {
                self.parking.parked = None;
                self.launch = launch;
                self.disconnect = None;
                self.failed_before_kill = false;
                let _live = self.route.send_replace(Route::Live(self.input.clone()));
                self.life
                    .publish(&mut self.agent, SessionEvent::Woken)
                    .await;
                self.parking.touch();
                tracing::debug!("woke the session's agent");
                Ok(())
            }
            Ok(Err(failure)) => {
                let message = failure.to_string();
                tracing::info!(%message, "the parked session's agent did not start again");
                self.outputs_open = false;
                self.enter_parked(ParkReason::wake_failed(&message)).await;
                Err(SessionError::WakeFailed(message))
            }
            Err(_stopped) => {
                // Killed or the node stopping while it started: the new
                // agent got the stop, and the loop waits for its exit.
                self.parking.parked = None;
                Err(SessionError::Ended)
            }
        }
    }

    /// Whether the last wake failed: a prompt the node delivers on its own
    /// (`queue`) then waits in the queue instead of trying again.
    pub(crate) const fn wake_failed(&self) -> bool {
        matches!(
            self.parking.parked,
            Some(Parked {
                reason: ParkReason::WakeFailed { .. },
                ..
            })
        )
    }
}
