//! [`SessionHandle`]: the cheap, cloneable view of one session. What it
//! sends the session is `input`'s.

mod input;
mod title;
pub use title::TitleUpdate;
pub(crate) use title::{Rename, RenameChange};

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use vessel_agent::{AgentInput, Driver};
use vessel_proc::ProcessLifetime;
use vessel_proto::{
    Conversation, EventSeq, ModelCall, OptionChoice, SessionId, SessionInfo, SessionOption,
    SessionTitle, TurnPoint,
};
use vessel_pty::PtyInput;

use crate::conversation::Seen;
use crate::error::ParkRefusal;
use crate::error::{ModeKind, SessionError};
use crate::forge::ForgeView;
use crate::marks::Marks;
use crate::moving::Control;
use crate::parking::{ParkOrder, Route};
use crate::ring::{Follow, FollowError, Ring};

/// State shared between a session's actor and its handles.
///
/// Everything here is either immutable, a channel end, or the ring behind
/// its own short lock; all other session state is owned by the actor.
#[derive(Debug)]
pub(crate) struct Shared {
    pub(crate) id: SessionId,
    /// Local-only account/location snapshot, changed with the actual launch.
    pub(crate) limits: watch::Sender<crate::LimitAssociation>,
    /// Latest info, published by the actor.
    pub(crate) info: watch::Receiver<SessionInfo>,
    /// Recent records and the live stream. A `std` mutex: it is held only
    /// for an append or a snapshot, never across `.await`.
    pub(crate) ring: Mutex<Ring>,
    /// Where input goes, by mode.
    pub(crate) input: Input,
    /// Cancelled to stop the session (a child of the runtime's shutdown
    /// token).
    pub(crate) kill: CancellationToken,
    /// Orders for the actor (structured sessions: moves).
    pub(crate) control: Option<mpsc::Sender<Control>>,
    /// The protocol its agent speaks (ADR 0057), for its life: what a move
    /// builds the new agent's command for. ACP for a terminal session.
    pub(crate) driver: Driver,
    /// Model calls the node's proxy made for the session, for the actor to
    /// record (protocol 7). Bounded; see [`SessionHandle::model_call`].
    pub(crate) model_calls: mpsc::Sender<ModelCall>,
    /// Set just before the final status is published (`Publisher::commit`),
    /// so whoever sees that record finds it set ([`SessionHandle::has_ended`]).
    pub(crate) ended: AtomicBool,
    /// Renames for the actor to publish (protocol 9). Bounded; a caller
    /// waits for room, then for the rename to be in the log.
    pub(crate) renames: mpsc::Sender<Rename>,
    /// The options of the last `options_changed` the session published
    /// (protocol 9), set by the publisher; what a move or an export carries
    /// on. A `std` mutex held for one clone or one store, never across an
    /// `.await`.
    pub(crate) reported: Mutex<Option<Arc<[SessionOption]>>>,
    /// The conversation as the log says it now (protocol 10), kept by the
    /// publisher; what `session.conversation` answers. A `std` mutex held
    /// for one update or one snapshot, never across an `.await`.
    pub(crate) seen: Mutex<Seen>,
    /// The session's marks (protocol 10, `marks`): the store's copy, set by
    /// the node, and the attention point, set by the publisher. A `std`
    /// mutex held for one assignment or one copy, never across an
    /// `.await`.
    pub(crate) marks: Mutex<Marks>,
    /// The session's pull request as the node gives it (protocol 11,
    /// `forge`): the latest wins, the actor reads it.
    pub(crate) forge: watch::Sender<Option<Arc<ForgeView>>>,
    /// The process id of the session's program (its PTY's or its agent's),
    /// replaced when a move starts another agent (protocol 11: the ports a
    /// session listens on are found from it, ADR 0031).
    pub(crate) pid: AtomicU32,
    /// Observer from the actual runner, never minted from this numeric PID.
    pub(crate) process: Mutex<Option<ProcessLifetime>>,
}

impl Shared {
    /// The ring, recovering from a poisoned lock: a panic elsewhere cannot
    /// leave a half-appended record (append is push then send), so the data
    /// is still consistent.
    pub(crate) fn ring(&self) -> MutexGuard<'_, Ring> {
        self.ring.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The input path of a session, chosen by its mode.
///
/// Input goes straight to the process side (the PTY's bounded input queue,
/// or the ACP driver), not through the actor, so a program that stops
/// reading input slows down only the caller sending it, never the actor's
/// output pump or `kill`.
#[derive(Debug)]
pub(crate) enum Input {
    Terminal(PtyInput),
    /// Where the current agent's input goes: to it, nowhere while it is
    /// replaced (a move, a park), or to the actor while it is parked
    /// (protocol 12).
    Structured(watch::Receiver<Route>),
}

impl Input {
    const fn mode(&self) -> ModeKind {
        match self {
            Self::Terminal(_) => ModeKind::Terminal,
            Self::Structured(_) => ModeKind::Structured,
        }
    }
}

/// The agent input goes to now; refused while the session moves.
fn current(route: &watch::Receiver<Route>) -> Result<AgentInput, SessionError> {
    route.borrow().input()
}

/// A handle to one session. Cheap to clone; every clone refers to the same
/// session, which keeps running whether or not any handle exists.
#[derive(Debug, Clone)]
pub struct SessionHandle(pub(crate) Arc<Shared>);

impl SessionHandle {
    /// Saved-sign-in association of the actual launch, without credentials.
    #[must_use]
    pub fn limit_association(&self) -> crate::LimitAssociation {
        self.0.limits.borrow().clone()
    }

    /// The session's id.
    #[must_use]
    pub fn id(&self) -> SessionId {
        self.0.id
    }

    /// The protocol its agent speaks (ADR 0057).
    #[must_use]
    pub fn driver(&self) -> Driver {
        self.0.driver
    }

    /// The choices the session runs with now (protocol 9): every select's
    /// current value and every toggle's state from its last
    /// `options_changed`; before any, its spec's own choices; none for a
    /// terminal session. What a session continuing elsewhere (another
    /// profile, another node) applies after it is resumed, so it keeps its
    /// mode rather than taking the agent's default.
    #[must_use]
    pub fn choices(&self) -> Vec<OptionChoice> {
        let reported = self
            .0
            .reported
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        match reported {
            Some(options) => options.iter().filter_map(SessionOption::choice).collect(),
            None => self.0.info.borrow().spec.mode.options().to_vec(),
        }
    }

    /// The agent's options as it last reported them (protocol 9). Waits
    /// for the handshake while the session starts.
    ///
    /// # Errors
    /// [`SessionError::NoOptions`] for a terminal session,
    /// [`SessionError::Moving`] during a move, [`SessionError::Ended`] once
    /// the session has ended.
    pub async fn options(&self) -> Result<Vec<SessionOption>, SessionError> {
        // A parked agent's options are what it last reported (protocol 12):
        // reading them is no reason to start it.
        if self.parked() {
            let reported = self
                .0
                .reported
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            return Ok(reported.map(|options| options.to_vec()).unwrap_or_default());
        }
        Ok(self.agent()?.options().await?)
    }

    /// Change one of the agent's options (protocol 9); answers every option
    /// once the agent has. The change is in the log as `options_changed`.
    ///
    /// # Errors
    /// As [`options`](Self::options), and [`SessionError::Input`] with the
    /// choice the agent does not offer or its refusal.
    pub async fn set_option(
        &self,
        choice: OptionChoice,
    ) -> Result<Vec<SessionOption>, SessionError> {
        // Changing an option needs the agent: a parked one is woken first
        // (protocol 12).
        if self.parked() {
            let _woken = self.wake().await?;
        }
        Ok(self.agent()?.set_option(choice).await?)
    }

    /// Stop the session's agent now, keeping the session and its
    /// conversation (protocol 12's `session.park`); the info after.
    ///
    /// # Errors
    /// [`SessionError::NotParked`] when it is not idle or its agent cannot
    /// resume, [`SessionError::Moving`], [`SessionError::Ended`].
    pub async fn park(&self) -> Result<SessionInfo, SessionError> {
        let route = self.structured_route(ParkRefusal::Terminal)?;
        self.order(route, |reply| Control::Park(ParkOrder::Park(reply)))
            .await?;
        Ok(self.info())
    }

    /// Start a parked session's agent again with its conversation, once it
    /// is ready (protocol 12's `session.wake`); at once when it is not
    /// parked. The info after.
    ///
    /// # Errors
    /// [`SessionError::WakeFailed`] (it stays parked),
    /// [`SessionError::Moving`], [`SessionError::Ended`].
    pub async fn wake(&self) -> Result<SessionInfo, SessionError> {
        let route = self.structured_route(ParkRefusal::Terminal)?;
        self.order(route, |reply| Control::Park(ParkOrder::Wake(reply)))
            .await?;
        Ok(self.info())
    }

    /// Take the conversation back to the end of the turn `point` names
    /// (protocol 14's `session.rewind`, ADR 0052): the agent's conversation
    /// is forked there and the session goes on with the fork. Between turns
    /// only; a parked session is woken first. The info after.
    ///
    /// # Errors
    /// [`SessionError::NotRewound`] while a turn runs, something waits or
    /// the queue moves, or for a terminal session;
    /// [`SessionError::ForkRefused`] when the agent cannot fork there;
    /// [`SessionError::RewindFailed`] (the old conversation goes on);
    /// [`SessionError::Moving`], [`SessionError::Ended`].
    pub async fn rewind(
        &self,
        point: TurnPoint,
        after: EventSeq,
        files_changed: u32,
    ) -> Result<SessionInfo, SessionError> {
        if !self.0.info.borrow().status.is_live() {
            return Err(SessionError::Ended);
        }
        let Input::Structured(route) = &self.0.input else {
            return Err(crate::error::RewindRefusal::Terminal.into());
        };
        self.order(route, |reply| {
            Control::Rewind(crate::forking::RewindOrder {
                point,
                after,
                files_changed,
                reply,
            })
        })
        .await?;
        Ok(self.info())
    }

    /// Whether the session's agent is parked now.
    fn parked(&self) -> bool {
        matches!(&self.0.input, Input::Structured(route) if matches!(*route.borrow(), Route::Parked))
    }

    /// The conversation as the session's log says it now (protocol 10):
    /// the queue, the agent's commands, context and abilities, and what
    /// waits on an answer. Kept as the session publishes; no log is read.
    ///
    /// # Errors
    /// [`SessionError::Terminal`] for a terminal session, whose harness
    /// runs its own interface.
    pub fn conversation(&self) -> Result<Conversation, SessionError> {
        match &self.0.input {
            Input::Terminal(_) => Err(SessionError::Terminal("a conversation")),
            Input::Structured(_) => Ok(self
                .0
                .seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .conversation()),
        }
    }

    /// Recover one current inbox form without cloning unrelated requests.
    ///
    /// # Errors
    /// [`SessionError::Terminal`] for a terminal session.
    pub fn inbox_request(
        &self,
        request: vessel_proto::inbox::InboxRequest,
    ) -> Result<Option<vessel_proto::inbox::InboxItem>, SessionError> {
        match &self.0.input {
            Input::Terminal(_) => Err(SessionError::Terminal("an inbox request")),
            Input::Structured(_) => Ok(self
                .0
                .seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .inbox_request(request)),
        }
    }

    /// Give the session `title` (protocol 9): a `renamed` record, in order
    /// with the rest of its log, then the info as it is with it.
    ///
    /// # Errors
    /// [`SessionError::Ended`] once the session has ended.
    pub async fn rename(&self, title: SessionTitle) -> Result<SessionInfo, SessionError> {
        let _applied = self.rename_change(RenameChange::Typed(title)).await?;
        Ok(self.info())
    }

    /// Give the session what the node knows of its pull request now
    /// (protocol 11, ADR 0029), `None` when it no longer watches it. The
    /// actor puts the summary in the info and the facts in its board, as
    /// the latest value of a watch: a view given twice before the actor
    /// looks is read once, and an unchanged one wakes nothing. Never
    /// waits; ignored once the session has ended.
    pub fn set_pull_request(&self, view: Option<ForgeView>) {
        if self.has_ended() {
            return;
        }
        let view = view.map(Arc::new);
        let _changed = self.0.forge.send_if_modified(|current| {
            if *current == view {
                return false;
            }
            *current = view;
            true
        });
    }

    /// The live agent's input, for the option calls.
    fn agent(&self) -> Result<AgentInput, SessionError> {
        if !self.0.info.borrow().status.is_live() {
            return Err(SessionError::Ended);
        }
        match &self.0.input {
            Input::Terminal(_) => Err(SessionError::NoOptions),
            Input::Structured(route) => current(route),
        }
    }

    /// Whether the session has ended: true from just before its final
    /// status (`Exited`, `Failed`, `Interrupted`) is visible to followers,
    /// readers and info watchers, so anyone who saw it end finds this true.
    /// What frees an ended session's places (the node's agent limits) checks
    /// this rather than waiting for the actor to finish.
    #[must_use]
    pub fn has_ended(&self) -> bool {
        self.0.ended.load(Ordering::Acquire)
    }

    /// The process id of its program while it runs: the root of the
    /// process tree whose listening ports `port.list` reports (ADR 0031).
    /// `None` once it ended (the id may be another process's by then).
    #[must_use]
    pub fn pid(&self) -> Option<u32> {
        let pid = self.0.pid.load(Ordering::Acquire);
        (pid != 0 && !self.has_ended()).then_some(pid)
    }

    /// Observe the current runner issuance; an ended or revoked run grants none.
    #[must_use]
    pub fn process_lifetime(&self) -> Option<ProcessLifetime> {
        if self.has_ended() {
            return None;
        }
        self.0
            .process
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .filter(|process| process.is_live())
            .cloned()
    }

    /// Record one model call the node's proxy made for this session
    /// (protocol 7): the actor publishes it as a `ModelCall` event, in
    /// order with the session's other records. Never waits: a call that
    /// finds the queue full, or the session ended, is dropped with a
    /// warning, so a slow log never holds up the proxy.
    pub fn model_call(&self, call: ModelCall) {
        if self.has_ended() {
            tracing::debug!(session = %self.id(), "model call after the session ended; not recorded");
            return;
        }
        if let Err(error) = self.0.model_calls.try_send(call) {
            tracing::warn!(session = %self.id(), %error, "model call not recorded");
        }
    }

    /// A snapshot of the session's info (one clone of the current value),
    /// with its marks (protocol 10): the store's, and `unread` from the
    /// records published so far; an archived session in the archive.
    #[must_use]
    pub fn info(&self) -> SessionInfo {
        let mut info = self.0.info.borrow().clone();
        // The watch's lock is released before the marks' is taken: neither
        // is held while the other is.
        self.0.marks().apply(&mut info);
        info
    }

    /// A receiver of info changes.
    ///
    /// It is notified when the status, the harness session id, the profile
    /// or the session's place on the board (its lane, phrase or activity)
    /// changes. The value's `next_seq` and the board's `last_activity_at`
    /// are kept current too, but advancing them alone does not wake
    /// watchers (they change with every event, or every burst of terminal
    /// output). The sender closes once the session has published its final
    /// status. Its values carry no marks, which change apart from the
    /// session's records: [`info`](Self::info) adds them.
    #[must_use]
    pub fn watch_info(&self) -> watch::Receiver<SessionInfo> {
        self.0.info.clone()
    }

    /// Wait until the session has ended, then return its final info.
    ///
    /// Resolves once the actor has published the final status and let go
    /// of everything it publishes through (ring, durable channel, info), so
    /// afterwards [`SessionRuntime::remove`](crate::SessionRuntime::remove)
    /// accepts the session. Returns at once for a session that already
    /// ended. Never resolves for a session nobody stops; `kill` first if it
    /// should end.
    pub async fn ended(&self) -> SessionInfo {
        let mut info = self.0.info.clone();
        // `changed` fails only once the actor dropped the sender.
        while info.changed().await.is_ok() {}
        self.info()
    }

    /// Whether the session can publish no more records.
    pub(crate) fn is_ended(&self) -> bool {
        self.0.ring().is_closed()
    }

    /// Stop the session: ask the process to exit, then kill its process
    /// group after a grace period. Returns immediately; the final status
    /// arrives as the session's last event.
    pub fn kill(&self) {
        self.0.kill.cancel();
    }

    /// Records from `from` onwards: the cached part now, the rest live.
    ///
    /// # Errors
    /// [`FollowError::Gap`] when `from` is older than the ring (the node
    /// then reads `from..oldest_cached` from the store and calls `follow`
    /// again), [`FollowError::Ahead`] when `from` is past the end of the
    /// log.
    pub fn follow(&self, from: EventSeq) -> Result<Follow, FollowError> {
        self.0.ring().follow(from)
    }
}
