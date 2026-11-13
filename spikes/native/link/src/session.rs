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
use bytes::Bytes;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc;
use vessel_app_view::terminal::Outgoing;
#[cfg(not(target_arch = "wasm32"))]
use vessel_client::NodeClient;
use vessel_proto::conversation::SessionDeleted;
use vessel_proto::{ErrorCode, Mark, ProfileName, SessionId, SessionInfo, SessionTitle};
#[cfg(not(target_arch = "wasm32"))]
use vessel_proto::{SessionInput, TerminalBytes, TerminalInput, TerminalSize};

// The acts' types are the screens' and every target's; carrying them out
// takes a node client, which a browser build does not have yet
// (docs/plans/backend-m12-browser.md).
#[cfg(not(target_arch = "wasm32"))]
use crate::{Event, LinkError, State};

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
}

impl Act {
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
        let events = self.events.clone();
        let machine = machine.to_owned();
        let client = self
            .clients
            .get(&machine)
            .map(|connected| connected.client.clone());
        drop(tokio::spawn(async move {
            let result = match client {
                Some(client) => run(&client, session, act).await,
                None => Err(ActFailed {
                    code: None,
                    message: LinkError::NotConnected(machine.clone()).to_string(),
                }),
            };
            answer(
                &events,
                ActAnswer {
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

#[cfg(not(target_arch = "wasm32"))]
async fn run(client: &NodeClient, session: SessionId, act: Act) -> Result<Acted, ActFailed> {
    let boxed = |info: SessionInfo| Acted::Session(Box::new(info));
    let done = match act {
        Act::Info => client.session(session).await.map(boxed),
        Act::Rename(title) => client.rename(session, title).await.map(boxed),
        Act::Mark(mark) => client.mark(session, mark).await.map(boxed),
        Act::StartAgain => client.resume_session(session).await.map(boxed),
        Act::Delete => client.delete_session(session).await.map(Acted::Deleted),
        Act::Kill => client.kill(session).await.map(|()| Acted::Done),
        Act::MoveProfile(profile) => client
            .move_profile(session, profile, false)
            .await
            .map(boxed),
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
    done.map_err(|error| ActFailed {
        code: error.code(),
        message: error.to_string(),
    })
}
