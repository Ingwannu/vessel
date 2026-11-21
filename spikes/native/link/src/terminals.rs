//! Shell terminals for a workspace's terminal tabs (ADR 0031's calls,
//! protocol 11; TRACKER 481, 482): open one, list a place's, follow one's
//! output, send it keys and sizes, close it.
//!
//! [Decision Log]
//! - Purpose: Paseo's workspace opens a shell as a tab, streams it, types
//!   into it and kills it on close (`use-workspace-terminals.ts`,
//!   `terminal-stream-controller.ts`). The node has had the calls since
//!   protocol 11; the app had none of them.
//! - Existing assets: `diagnostics` and `node_settings` (a task per call,
//!   the answer as an event, failures as codes only) and `session::follow`
//!   (a stream's records as events). `vessel-client`'s `follow_terminal`
//!   pages a terminal's history, then follows it live.
//! - Choice: one request enum and one answer enum, in that shape. A follow
//!   is a task per terminal, kept in [`Hub`] so a second follow of the same
//!   terminal replaces the first and a close or an unfollow stops it. Each
//!   unary call is bounded and runs as its own task. Order of keys is the
//!   screen's: its `TerminalSender` keeps one send in flight per terminal.
//! - Cost: no automatic reattach. A follow that drops reports `Lost`, and
//!   the screen follows again from its cursor when the machine is back.
//! - Several clients (ADR 0116, protocol 37): a list may wait for the
//!   node's revision to move, and a size names a claim or an update. Both
//!   are left out for an older node, which would refuse them; the answer
//!   says whether the node waits, so the screen never re-asks a node that
//!   answers at once.

use vessel_app_view::terminal::Outgoing;
use vessel_app_view::terminal_tabs::Failure;
use vessel_proto::terminals::{TerminalInfo, TerminalList, TerminalPlace};
use vessel_proto::{EventRecord, EventSeq, TerminalId};

/// What is asked about a machine's terminals.
#[derive(Debug, Clone)]
pub enum Request {
    /// Open the machine's shell in `place` (`terminal.create`), or one of
    /// its terminal profiles (ADR 0117).
    Create {
        /// Where: the workspace's place.
        place: TerminalPlace,
        /// The profile to run instead of the shell, by id.
        profile: Option<vessel_proto::terminal_profiles::Id>,
    },
    /// The place's terminals (`terminal.list`).
    List {
        /// Which place.
        place: TerminalPlace,
        /// The list's revision last seen: the node answers when it moves
        /// (ADR 0116's long poll, at most 30 s). Dropped for a node below
        /// `SHARED_TERMINALS_PROTOCOL`, which answers at once.
        changed_from: Option<u64>,
    },
    /// Every terminal the caller may use (`terminal.list` with no place):
    /// a worktree's removal counts the live ones in its folder, whatever
    /// their place (ADR 0121).
    ListAll,
    /// Follow a terminal's output from `from` (`terminal.subscribe`, its
    /// history paged first), replacing an earlier follow of it.
    Follow {
        /// Which terminal.
        terminal: TerminalId,
        /// The first record wanted.
        from: EventSeq,
    },
    /// Stop following a terminal.
    Unfollow {
        /// Which terminal.
        terminal: TerminalId,
    },
    /// Keys or a size (`terminal.send`), as the screen's sender took them.
    Send {
        /// Which terminal.
        terminal: TerminalId,
        /// What to send.
        outgoing: Outgoing,
    },
    /// End it if it runs and forget it (`terminal.close`); its follow stops.
    Close {
        /// Which terminal.
        terminal: TerminalId,
    },
}

/// One request to one machine.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The machine, by its book name.
    pub machine: String,
    /// What is asked.
    pub request: Request,
}

/// What a machine answered, each naming what it answers.
#[derive(Debug, Clone)]
pub enum Reply {
    /// A shell was opened in `place`, or why not.
    Created {
        /// Where it was asked for.
        place: TerminalPlace,
        /// The terminal.
        result: Result<Box<TerminalInfo>, Failure>,
    },
    /// The place's terminals, or why not.
    Listed {
        /// Which place.
        place: TerminalPlace,
        /// Its terminals.
        result: Result<TerminalList, Failure>,
        /// Whether the node waits on `changed_from` (protocol 37 or later),
        /// so the screen may ask again at once.
        waits: bool,
    },
    /// Every terminal, as `ListAll` asked.
    ListedAll {
        /// Its terminals.
        result: Result<TerminalList, Failure>,
    },
    /// A follow began: the terminal as it was then (records before its
    /// `next_seq` are history, the rest live).
    Following(Box<TerminalInfo>),
    /// A follow's next record.
    Record {
        /// Which terminal.
        terminal: TerminalId,
        /// The record.
        record: EventRecord,
    },
    /// The terminal ended: its follow is over.
    Ended {
        /// Which terminal.
        terminal: TerminalId,
    },
    /// A follow could not start, or stopped, without the terminal ending.
    Lost {
        /// Which terminal.
        terminal: TerminalId,
        /// Why.
        failure: Failure,
    },
    /// A send answered, well or not (a failed send is not repeated).
    Sent {
        /// Which terminal.
        terminal: TerminalId,
        /// Whether it was taken.
        result: Result<(), Failure>,
    },
    /// A close answered.
    Closed {
        /// Which terminal.
        terminal: TerminalId,
        /// Whether it was taken.
        result: Result<(), Failure>,
    },
}

/// A machine's answer.
#[derive(Debug, Clone)]
pub struct Answer {
    /// The machine asked.
    pub machine: String,
    /// What it answered.
    pub reply: Reply,
}

impl Ask {
    /// Its answer when it could not be asked at all: `failure` for a call,
    /// nothing for an unfollow.
    #[must_use]
    pub fn refused(self, failure: Failure) -> Option<Answer> {
        let reply = match self.request {
            Request::Create { place, .. } => Reply::Created {
                place,
                result: Err(failure),
            },
            Request::List { place, .. } => Reply::Listed {
                place,
                result: Err(failure),
                waits: false,
            },
            Request::ListAll => Reply::ListedAll {
                result: Err(failure),
            },
            Request::Follow { terminal, .. } => Reply::Lost { terminal, failure },
            Request::Send { terminal, .. } => Reply::Sent {
                terminal,
                result: Err(failure),
            },
            Request::Close { terminal } => Reply::Closed {
                terminal,
                result: Err(failure),
            },
            Request::Unfollow { .. } => return None,
        };
        Some(Answer {
            machine: self.machine,
            reply,
        })
    }
}

/// A size's intent as `protocol`'s node takes it: none below ADR 0116's
/// protocol, whose `Call::since` refuses one (an older node applies every
/// size, as a claim would).
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub(crate) fn intent_for(
    protocol: vessel_proto::ProtocolVersion,
    intent: vessel_proto::ResizeIntent,
) -> Option<vessel_proto::ResizeIntent> {
    waits_on_revisions(protocol).then_some(intent)
}

/// Whether `protocol`'s node waits on `terminal.list`'s `changed_from` and
/// takes a size's intent (ADR 0116).
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub(crate) fn waits_on_revisions(protocol: vessel_proto::ProtocolVersion) -> bool {
    vessel_proto::terminals::SHARED_TERMINALS_PROTOCOL.is_some_and(|since| protocol >= since)
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::Hub;
