//! What a person does to a session from its title bar, its Session
//! actions menu and its terminal, and the node's answers (the page's
//! `data/calls.ts` for the same calls).
//!
//! [Decision Log]
//! - Purpose: the session area renames, marks, starts again, deletes,
//!   kills, moves a session to another profile and types into a terminal,
//!   as the page does.
//! - Constraint: the link's task handles commands one after another; a
//!   call awaited there holds every other command (and the fleet's
//!   refresh) until the node answers.
//! - Choice: each act runs on a task of its own and answers with one
//!   `Event::Acted`, naming the act, so the screen puts a refusal beside
//!   the control it is about. Terminal input keeps its order without the
//!   link's help: the view's sender has one send in flight and waits for
//!   its answer before the next.
//! - Cost: two acts on one session can reach the node in either order
//!   (a pin and an archive clicked in one breath); each answers with the
//!   session as it then stands, and the list's next read settles it.

#[cfg(not(target_arch = "wasm32"))]
mod stream;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use stream::follow;

use vessel_app_view::terminal::Outgoing;
use vessel_app_view::timeline::Until;
use vessel_proto::conversation::{Conversation, QueueChange, SessionDeleted};
use vessel_proto::{
    ErrorCode, EventRecord, EventSeq, Mark, OptionChoice, ProfileName, SessionId, SessionInfo,
    SessionTitle,
};
use vessel_proto::{PermissionId, PermissionOptionId, QuestionAnswer, QuestionId};

// The acts run on the desktop's runtime through `vessel-client`; a browser
// has neither (its demo refuses an act, `web.rs`), so only the acts' types
// build for wasm32.
#[cfg(not(target_arch = "wasm32"))]
use bytes::Bytes;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use vessel_client::NodeClient;
#[cfg(not(target_arch = "wasm32"))]
use vessel_proto::{PermissionOutcome, SessionInput, TerminalBytes, TerminalInput, TerminalSize};

#[cfg(not(target_arch = "wasm32"))]
use crate::{Event, LinkError, State};
mod terminal_opening;
#[cfg(target_arch = "wasm32")]
pub use crate::web::NodeName;
pub use terminal_opening::TerminalOpening;
#[cfg(not(target_arch = "wasm32"))]
pub use vessel_client::book::NodeName;

/// An act on one session.
#[derive(Debug)]
pub enum Act {
    /// Read it as the node lists it (its marks, abilities, title).
    Info,
    /// Rename it (protocol 9).
    Rename(SessionTitle),
    /// Pin, unpin, archive, unarchive, mark read or unread (protocol 10).
    Mark(Mark),
    /// Start an ended conversation again (protocol 10).
    StartAgain,
    /// Delete an ended session for good (protocol 10).
    Delete,
    /// Stop the agent and everything it started.
    Kill,
    /// Move a live conversation to another profile (protocol 5); `None`:
    /// the default.
    MoveProfile(Option<ProfileName>),
    /// Keys or a size for a terminal session.
    Terminal(Outgoing),
    /// Change the prompts waiting their turn (protocol 10): edit, move,
    /// remove, send one now, hold, resume or clear.
    Queue(QueueChange),
    /// Answer a question the agent asks (protocol 10): the form filled
    /// in, or declined. The first answer wins.
    Answer(QuestionId, QuestionAnswer),
    /// Answer a permission request with one of its options; the act's
    /// answer says whether it went (the card shows it being sent, or why
    /// it could not be).
    Permit(PermissionId, PermissionOptionId),
    /// Read what the session holds now (`session.conversation`): the
    /// requests and questions waiting, whatever its log still keeps.
    Conversation,
    /// Change one of a running session's options (`session.set_option`,
    /// protocol 9): its model, effort, permission mode or a feature. The
    /// options as they then stand come back as the session's own
    /// `options_changed` record; the agent's refusal (one it cannot make
    /// mid-turn, a value it does not take) is the act's failure.
    SetOption(OptionChoice),
    /// Start a parked session's agent again (`session.wake`, protocol 12):
    /// the session as it then stands, its `parked` cleared.
    Wake,
    /// Take the conversation back to the end of the turn whose
    /// `turn_ended` record is this one (`session.rewind`, protocol 14):
    /// the session as it then stands; its `rewound` record follows in the
    /// log.
    Rewind(EventSeq),
    /// A new session continuing this one's conversation from the end of
    /// the turn whose `turn_ended` record is this one, or as it is now
    /// (`session.fork`, protocol 14): the new session, as `session.create`
    /// answers it; its `forked` record opens its log.
    Fork(Option<EventSeq>),
    /// Read its log from the start up to `Until` (`session.read`, page by
    /// page): a fork's origin, for what the fork carries over (F1b).
    Log(Until),
    /// An app terminal's history, owned by one particular opening.
    TerminalLog {
        /// Echoed unchanged on success and refusal, including a closed link.
        opening: TerminalOpening,
        /// The same cutoff as an ordinary log read.
        until: Until,
    },
}

/// Which act an answer is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActKind {
    /// A read.
    Info,
    /// A rename.
    Rename,
    /// A mark.
    Mark,
    /// Start again.
    StartAgain,
    /// A delete.
    Delete,
    /// A kill.
    Kill,
    /// A move to another profile.
    MoveProfile,
    /// Terminal input.
    Terminal,
    /// A change to the queue.
    Queue,
    /// An answer to a question.
    Answer,
    /// An answer to a permission request.
    Permit,
    /// A read of what the session holds now.
    Conversation,
    /// A change to one of its options.
    SetOption,
    /// A parked agent started again.
    Wake,
    /// A rewind.
    Rewind,
    /// A fork.
    Fork,
    /// A read of its log.
    Log,
}

impl Act {
    /// Ownership to echo even when the call is refused before reaching a node.
    #[must_use]
    pub fn terminal_opening(&self) -> Option<&TerminalOpening> {
        match self {
            Self::TerminalLog { opening, .. } => Some(opening),
            Self::Info
            | Self::Rename(_)
            | Self::Mark(_)
            | Self::StartAgain
            | Self::Delete
            | Self::Kill
            | Self::MoveProfile(_)
            | Self::Terminal(_)
            | Self::Queue(_)
            | Self::Answer(..)
            | Self::Permit(..)
            | Self::Conversation
            | Self::SetOption(_)
            | Self::Wake
            | Self::Rewind(_)
            | Self::Fork(_)
            | Self::Log(_) => None,
        }
    }

    /// Which act this is.
    #[must_use]
    pub const fn kind(&self) -> ActKind {
        match self {
            Self::Info => ActKind::Info,
            Self::Rename(_) => ActKind::Rename,
            Self::Mark(_) => ActKind::Mark,
            Self::StartAgain => ActKind::StartAgain,
            Self::Delete => ActKind::Delete,
            Self::Kill => ActKind::Kill,
            Self::MoveProfile(_) => ActKind::MoveProfile,
            Self::Terminal(_) => ActKind::Terminal,
            Self::Queue(_) => ActKind::Queue,
            Self::Answer(..) => ActKind::Answer,
            Self::Permit(..) => ActKind::Permit,
            Self::Conversation => ActKind::Conversation,
            Self::SetOption(_) => ActKind::SetOption,
            Self::Wake => ActKind::Wake,
            Self::Rewind(_) => ActKind::Rewind,
            Self::Fork(_) => ActKind::Fork,
            Self::Log(_) | Self::TerminalLog { .. } => ActKind::Log,
        }
    }
}

/// What a session act came to.
#[derive(Debug)]
pub enum Acted {
    /// The session as it now stands (a read, a rename, a mark, start
    /// again, a move).
    Session(Box<SessionInfo>),
    /// It was deleted; what became of its worktree.
    Deleted(SessionDeleted),
    /// It was killed, or the terminal took the input.
    Done,
    /// What the session holds now.
    Conversation(Box<Conversation>),
    /// The new session a fork made (not the session acted on, whose
    /// description stands).
    Forked(Box<SessionInfo>),
    /// Its log's records, in order, up to where the read stopped.
    Log(Vec<EventRecord>),
}

/// Why a session act failed: the node's code, when it answered with one,
/// and its words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActFailed {
    /// The node's code (`Conflict`, `Unsupported`, ...); `None` when the
    /// connection failed instead.
    pub code: Option<ErrorCode>,
    /// What to show.
    pub message: String,
}

/// The answer to one act.
#[derive(Debug)]
pub struct ActAnswer {
    /// Terminal history ownership; absent for ordinary conversation acts.
    pub terminal: Option<TerminalOpening>,
    /// The machine.
    pub machine: String,
    /// The session.
    pub session: SessionId,
    /// Which act.
    pub kind: ActKind,
    /// What it came to.
    pub result: Result<Acted, ActFailed>,
}

#[cfg(not(target_arch = "wasm32"))]
impl State {
    /// `act` on `session`, on a task of its own; its answer comes back as
    /// `Event::Acted`.
    pub(crate) fn act(&self, machine: &str, session: SessionId, act: Act) {
        let kind = act.kind();
        let terminal = act.terminal_opening().cloned();
        let events = self.events.clone();
        let projection = self.session_projection.clone();
        let machine = machine.to_owned();
        let client = self
            .clients
            .get(&machine)
            .map(|connected| connected.client.clone());
        drop(tokio::spawn(async move {
            let result = match client {
                Some(client) => {
                    let result = run(&client, session, act).await;
                    projection.after(kind);
                    result
                }
                None => Err(ActFailed {
                    code: None,
                    message: LinkError::NotConnected(machine.clone()).to_string(),
                }),
            };
            answer(
                &events,
                ActAnswer {
                    terminal,
                    machine,
                    session,
                    kind,
                    result,
                },
            )
            .await;
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn answer(events: &mpsc::Sender<Event>, answer: ActAnswer) {
    // The receiver is gone only when the app is closing.
    let _closed = events.send(Event::Acted(answer)).await;
}

/// Records of `session`'s log read per page.
#[cfg(not(target_arch = "wasm32"))]
const PAGE: u32 = 512;

/// `session`'s log from its start, page by page, up to the first record
/// `until` does not keep (the log is in order) or its end.
#[cfg(not(target_arch = "wasm32"))]
async fn read_until(
    client: &NodeClient,
    session: SessionId,
    until: Until,
) -> Result<Vec<EventRecord>, vessel_client::CallError> {
    let mut kept = Vec::new();
    let mut from = EventSeq::new(0);
    loop {
        let page = client.read(session, from, PAGE).await?;
        // A page that does not move on is the end too (nothing past it).
        let last = page.at_end || page.next <= from;
        for record in page.records {
            if !until.keeps(&record) {
                return Ok(kept);
            }
            kept.push(record);
        }
        if last {
            return Ok(kept);
        }
        from = page.next;
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn run(
    client: &NodeClient,
    session: SessionId,
    act: Act,
) -> Result<Acted, ActFailed> {
    let boxed = |info: SessionInfo| Acted::Session(Box::new(info));
    let done = match act {
        Act::Info => client.session(session).await.map(boxed),
        Act::Rename(title) => client.rename(session, title).await.map(boxed),
        Act::Mark(mark) => client.mark(session, mark).await.map(boxed),
        Act::StartAgain => client.resume_session(session).await.map(boxed),
        Act::Wake => client
            .call(vessel_proto::parking::WakeSession { session })
            .await
            .map(boxed),
        Act::Rewind(after) => client
            .call(vessel_proto::forking::RewindSession { session, after })
            .await
            .map(boxed),
        Act::Fork(after) => client
            .call(vessel_proto::forking::ForkSession { session, after })
            .await
            .map(|info| Acted::Forked(Box::new(info))),
        Act::Log(until) | Act::TerminalLog { until, .. } => {
            read_until(client, session, until).await.map(Acted::Log)
        }
        Act::Delete => client.delete_session(session).await.map(Acted::Deleted),
        Act::Kill => client.kill(session).await.map(|()| Acted::Done),
        // The options as they now stand also arrive as the session's own
        // `options_changed` record, which the conversation folds.
        Act::SetOption(choice) => client
            .set_option(session, choice)
            .await
            .map(|_options| Acted::Done),
        Act::Conversation => client
            .conversation(session)
            .await
            .map(|held| Acted::Conversation(Box::new(held))),
        Act::MoveProfile(profile) => client
            .move_profile(session, profile, false)
            .await
            .map(boxed),
        // The queue as it stands afterwards comes back as the session's
        // own `queue_changed` record, which the conversation follows.
        Act::Queue(change) => client
            .change_queue(session, change)
            .await
            .map(|_changed| Acted::Done),
        // The question settles as the session's own `question_resolved`
        // record; a refusal (another client answered first, or a value
        // that does not fit) comes back as the act's failure.
        Act::Answer(id, answer) => client
            .send(session, SessionInput::AnswerQuestion { id, answer })
            .await
            .map(|()| Acted::Done),
        // The request settles as the session's own `permission_resolved`.
        Act::Permit(id, option) => client
            .send(
                session,
                SessionInput::AnswerPermission {
                    id,
                    outcome: PermissionOutcome::Selected { option },
                },
            )
            .await
            .map(|()| Acted::Done),
        Act::Terminal(outgoing) => {
            let input = match outgoing {
                Outgoing::Keys(text) => TerminalInput::Data {
                    data: TerminalBytes::new(Bytes::from(text.into_bytes())),
                },
                Outgoing::Resize { cols, rows } => TerminalInput::Resize {
                    size: TerminalSize { cols, rows },
                },
            };
            client
                .send(session, SessionInput::Terminal { input })
                .await
                .map(|()| Acted::Done)
        }
    };
    // A node's refusal is its own sentence (its code is beside it), not the
    // client's wrapping of it ("the node returned an error: Conflict: …").
    done.map_err(|error| ActFailed {
        code: error.code(),
        message: match &error {
            vessel_client::CallError::Remote(wire) => wire.message.clone(),
            vessel_client::CallError::Disconnected
            | vessel_client::CallError::UnexpectedReply(_)
            | vessel_client::CallError::RequestIdsExhausted
            | vessel_client::CallError::NotInProtocol { .. }
            | vessel_client::CallError::UnassignedProtocol { .. } => error.to_string(),
        },
    })
}
