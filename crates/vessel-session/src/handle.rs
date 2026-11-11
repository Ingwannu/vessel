//! [`SessionHandle`]: the cheap, cloneable view of one session.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use vessel_agent::AgentInput;
use vessel_proto::{
    Conversation, EventSeq, ModelCall, OptionChoice, SessionId, SessionInfo, SessionInput,
    SessionOption, SessionTitle, TerminalInput,
};
use vessel_pty::{PtyInput, WindowSize};

use crate::conversation::Seen;
use crate::error::{InputKind, ModeKind, SessionError};
use crate::lifecycle::Rename;
use crate::moving::Control;
use crate::ring::{Follow, FollowError, Ring};

/// State shared between a session's actor and its handles.
///
/// Everything here is either immutable, a channel end, or the ring behind
/// its own short lock; all other session state is owned by the actor.
#[derive(Debug)]
pub(crate) struct Shared {
    pub(crate) id: SessionId,
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
    /// The current agent's input; `None` while the session moves to another
    /// profile and its agent is replaced.
    Structured(watch::Receiver<Option<AgentInput>>),
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
fn current(route: &watch::Receiver<Option<AgentInput>>) -> Result<AgentInput, SessionError> {
    route.borrow().clone().ok_or(SessionError::Moving)
}

/// A handle to one session. Cheap to clone; every clone refers to the same
/// session, which keeps running whether or not any handle exists.
#[derive(Debug, Clone)]
pub struct SessionHandle(pub(crate) Arc<Shared>);

impl SessionHandle {
    /// The session's id.
    #[must_use]
    pub fn id(&self) -> SessionId {
        self.0.id
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
        Ok(self.agent()?.set_option(choice).await?)
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

    /// Give the session `title` (protocol 9): a `renamed` record, in order
    /// with the rest of its log, then the info as it is with it.
    ///
    /// # Errors
    /// [`SessionError::Ended`] once the session has ended.
    pub async fn rename(&self, title: SessionTitle) -> Result<SessionInfo, SessionError> {
        if !self.0.info.borrow().status.is_live() {
            return Err(SessionError::Ended);
        }
        let (done, renamed) = oneshot::channel();
        self.0
            .renames
            .send(Rename { title, done })
            .await
            .map_err(|_ended| SessionError::Ended)?;
        renamed.await.map_err(|_ended| SessionError::Ended)?;
        Ok(self.info())
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

    /// A snapshot of the session's info (one clone of the current value).
    #[must_use]
    pub fn info(&self) -> SessionInfo {
        self.0.info.borrow().clone()
    }

    /// A receiver of info changes.
    ///
    /// It is notified when the status, the harness session id, the profile
    /// or the session's place on the board (its lane, phrase or activity)
    /// changes. The value's `next_seq` and the board's `last_activity_at`
    /// are kept current too, but advancing them alone does not wake
    /// watchers (they change with every event, or every burst of terminal
    /// output). The sender closes once the session has published its final
    /// status.
    #[must_use]
    pub fn watch_info(&self) -> watch::Receiver<SessionInfo> {
        self.0.info.clone()
    }

    /// Send input to the session.
    ///
    /// Terminal input waits while the program's input queue is full.
    /// Structured input waits until the agent has applied it (and, while the
    /// ACP handshake runs, until it is ready).
    ///
    /// # Errors
    /// [`SessionError::Unsupported`] when the input does not match the
    /// session's mode, [`SessionError::Ended`] once the session has ended,
    /// [`SessionError::Input`] when the agent refuses it (for example a
    /// second answer to a permission request).
    pub async fn send(&self, input: SessionInput) -> Result<(), SessionError> {
        if !self.0.info.borrow().status.is_live() {
            return Err(SessionError::Ended);
        }
        match (&self.0.input, input) {
            (Input::Terminal(pty), SessionInput::Terminal { input }) => {
                let sent = match input {
                    TerminalInput::Data { data } => pty.write(data.into_bytes()).await,
                    TerminalInput::Resize { size } => {
                        pty.resize(WindowSize {
                            cols: size.cols,
                            rows: size.rows,
                        })
                        .await
                    }
                };
                sent.map_err(|vessel_pty::PtyClosed| SessionError::Ended)
            }
            (Input::Structured(route), SessionInput::Prompt { content }) => {
                Ok(current(route)?.prompt(content).await?)
            }
            (Input::Structured(route), SessionInput::CancelTurn { .. }) => {
                Ok(current(route)?.cancel_turn().await?)
            }
            (Input::Structured(route), SessionInput::AnswerPermission { id, outcome }) => {
                Ok(current(route)?.answer(id, outcome).await?)
            }
            (Input::Structured(_), SessionInput::AnswerQuestion { .. }) => {
                Err(SessionError::NotBuilt("answering a question"))
            }
            (
                input_path @ Input::Terminal(_),
                input @ (SessionInput::Prompt { .. }
                | SessionInput::CancelTurn { .. }
                | SessionInput::AnswerPermission { .. }
                | SessionInput::AnswerQuestion { .. }),
            )
            | (input_path @ Input::Structured(_), input @ SessionInput::Terminal { .. }) => {
                Err(SessionError::Unsupported {
                    input: InputKind::of(&input),
                    mode: input_path.mode(),
                })
            }
        }
    }

    /// Whether the session's agent is in the middle of a turn (a prompt
    /// already on its way counts); `false` for a terminal session, whose
    /// turns Vessel cannot see. For copying a session's files away
    /// (protocol 6's `session.export`): between turns its transcript is
    /// complete, during one the harness is still appending to it.
    ///
    /// # Errors
    /// [`SessionError::Moving`] while the session moves to another profile,
    /// [`SessionError::Ended`] once its agent's connection has closed.
    pub async fn turn_running(&self) -> Result<bool, SessionError> {
        match &self.0.input {
            Input::Terminal(_) => Ok(false),
            Input::Structured(route) => Ok(current(route)?.turn_running().await?),
        }
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
        info.borrow().clone()
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
