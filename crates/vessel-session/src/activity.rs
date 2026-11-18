//! What a session's agent is doing, read off the records the session
//! publishes, and the board placement that follows (protocol 8, ADR 0019).
//!
//! Owns: one session's worker facts (activity, when it was last seen doing
//! something, whether its latest turn was stopped, the permission requests
//! still open) and turning them into a `SessionBoard` with
//! [`board::derive()`]. Used only by the session's [`Publisher`], which shows
//! it every record it commits. Must not: publish anything, or read the
//! clock (callers pass the time of the record).
//!
//! [Decision Log]
//! - Purpose: the board needs to know whether an agent is working, waiting
//!   on its owner, between turns, or gone; `SessionStatus` says only
//!   whether its process lives.
//! - Constraint: the actor's loop already handles every output and must
//!   stay a pump; a structured session's moves replace its agent while the
//!   session goes on.
//! - Alternatives: hooks in each actor where it reads an output (two
//!   copies of the rules, and a move's replay or a drained output could
//!   slip past one); asking the agent's driver whether a turn runs (a round
//!   trip per question, and nothing for a terminal).
//! - Choice: the facts are a projection of the records, as the store's
//!   `sessions` row is. Every record passes the publisher, so this sees
//!   exactly what the log holds, in order, and nothing a move dropped. The
//!   one fact no record carries, a terminal's quiet, comes from the
//!   terminal actor's timer ([`Tracker::quiet`]).
//! - Rules (the plan's): `starting` is active. Structured: `Ready` and the
//!   end of a turn (`TurnEnded`, `TurnFailed`) make it idle; `TurnStarted`
//!   makes it active; a permission request or a question (protocol 10)
//!   makes it `waiting_input` until every one open is resolved, when the
//!   turn goes on (active). The
//!   latest turn is interrupted when it ended `cancelled`, until the next
//!   one starts. Terminal: output makes it active, quiet makes it idle. Any
//!   final status makes it `exited`: in building as `exited` (ADR 0026),
//!   not in the archive, which is for archived sessions only.
//! - Archived: the reducer's `is_terminated` (ADR 0026). No record says a
//!   session is archived, and a session is archived only once its records
//!   have ended (archiving a live one stops it first), so the facts here
//!   never hold it; the session's marks place an archived one in the
//!   archive when its info is read (`marks`), as the reducer would.
//! - Last activity: the time of the record that showed the agent doing
//!   something (the handshake done, a turn's start or end, a permission
//!   asked or answered, terminal output, the end). Message and tool-call
//!   chunks inside a turn do not move it: the contract lists what does, and
//!   a turn's start already says the agent is at work. A terminal's quiet
//!   does not move it either: quiet is the absence of activity.
//! - Pull requests (protocol 11, ADR 0029): the one input no record
//!   carries besides a terminal's quiet. The node reads them from its
//!   forge and gives them to the actor ([`Tracker::forge`]); they are held
//!   here so every later record's board has them, and dropped at the
//!   final status, before that record's board, so an ended session is
//!   placed as the store places a stored one.
//! - Must stay true: a record changes the facts the same way whichever
//!   actor published it, so a replay of the log gives the same facts (the
//!   quiet timer and the pull requests aside).

use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use vessel_proto::{
    Activity, AgentEvent, PermissionId, QuestionId, SessionBoard, SessionEvent, StopReason,
};

use crate::board::{self, Policy, PullRequestFacts, PullRequests, SessionFacts, WorkerFacts};
use crate::forge::ForgeView;

#[cfg(doc)]
use crate::publish::Publisher;

/// One session's worker facts.
#[derive(Debug)]
pub(crate) struct Tracker {
    activity: Activity,
    last_activity_at: DateTime<Utc>,
    /// The agent has been seen doing something since the session started.
    seen: bool,
    /// The latest turn ended cancelled, and none has started since.
    interrupted: bool,
    /// Permission requests and questions asked and not yet resolved.
    /// Rarely more than one; a set so a resolution matches its own request,
    /// whatever order several are answered in.
    pending: HashSet<Waiting>,
    /// The session's pull requests as the node last gave them: `None`
    /// while it cannot see them (`PullRequests::Unseen`).
    pulls: Option<Arc<[PullRequestFacts]>>,
    /// The loops the node drives for the session (its feedback switch).
    policy: Policy,
}

/// Something the agent waits on its owner for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Waiting {
    Permission(PermissionId),
    Question(QuestionId),
}

impl Tracker {
    /// A session created at `created_at`: starting, which is active.
    pub(crate) fn new(created_at: DateTime<Utc>) -> Self {
        Self {
            activity: Activity::Active,
            last_activity_at: created_at,
            seen: false,
            interrupted: false,
            pending: HashSet::new(),
            pulls: None,
            policy: Policy::default(),
        }
    }

    /// What the node knows of the session's pull requests now; `None`
    /// when it no longer watches it.
    pub(crate) fn forge(&mut self, view: Option<&ForgeView>) {
        self.pulls = view.and_then(|view| view.seen.clone());
        self.policy = view.map_or_else(Policy::default, |view| view.policy);
    }

    /// Read one record the session is publishing, stamped `at`.
    pub(crate) fn observe(&mut self, event: &SessionEvent, at: DateTime<Utc>) {
        match event {
            SessionEvent::Status { status } => {
                // `Starting` is where a tracker begins and `Running` says
                // nothing about turns; only the end changes the facts.
                if !status.is_live() {
                    self.pending.clear();
                    // Nothing watches an ended session (ADR 0029).
                    self.forge(None);
                    self.set(Activity::Exited, at);
                }
            }
            SessionEvent::Terminal(_) => self.set(Activity::Active, at),
            SessionEvent::Agent { event } => self.agent(event, at),
            SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            // A parked session sits where an idle one does (protocol 12):
            // nothing about its turns changed.
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. }
            | SessionEvent::Reloaded { .. } => {}
        }
    }

    fn agent(&mut self, event: &AgentEvent, at: DateTime<Utc>) {
        match event {
            // The handshake is done and no turn has started: between turns.
            // (A prompt sent while starting starts its turn after this.)
            AgentEvent::Ready { .. } => self.set(Activity::Idle, at),
            AgentEvent::TurnStarted { .. } => {
                self.interrupted = false;
                self.set(Activity::Active, at);
            }
            AgentEvent::PermissionRequested(request) => {
                self.waits(Waiting::Permission(request.id), at);
            }
            AgentEvent::QuestionAsked(question) => self.waits(Waiting::Question(question.id), at),
            AgentEvent::PermissionResolved { id, .. } => {
                self.resolved(Waiting::Permission(*id), at);
            }
            AgentEvent::QuestionResolved { id, .. } => self.resolved(Waiting::Question(*id), at),
            AgentEvent::TurnEnded { reason, .. } => {
                // A turn's requests end with it, answered or not.
                self.pending.clear();
                self.interrupted = *reason == StopReason::Cancelled;
                self.set(Activity::Idle, at);
            }
            AgentEvent::TurnFailed { .. } => {
                self.pending.clear();
                self.interrupted = false;
                self.set(Activity::Idle, at);
            }
            // A steer is the person adding to a turn already active.
            AgentEvent::UserMessageChunk { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. }
            | AgentEvent::HarnessLog { .. }
            | AgentEvent::Steered { .. } => {}
        }
    }

    /// The agent asks its owner something.
    fn waits(&mut self, waiting: Waiting, at: DateTime<Utc>) {
        let _new = self.pending.insert(waiting);
        self.set(Activity::WaitingInput, at);
    }

    /// What it asked was answered (or withdrawn).
    fn resolved(&mut self, waiting: Waiting, at: DateTime<Utc>) {
        // A resolution after the turn ended (the agent's driver withdraws
        // what a closed turn left open) changes nothing: not the activity,
        // not when the agent was last seen.
        if !self.pending.remove(&waiting) {
            return;
        }
        // The turn goes on once nothing waits on the owner.
        if self.pending.is_empty() && self.activity == Activity::WaitingInput {
            self.set(Activity::Active, at);
        } else {
            self.touch(at);
        }
    }

    /// A terminal session's output has been quiet for a while
    /// (`terminal::QUIET`): an active terminal becomes idle. Its last
    /// activity stays the last output.
    pub(crate) fn quiet(&mut self) {
        if self.activity == Activity::Active {
            self.activity = Activity::Idle;
        }
    }

    /// The session's place on the board as of `now`.
    ///
    /// Its pull requests and policy are the node's ([`Tracker::forge`]);
    /// no session has an artifact, so that fact is fixed here; and nothing
    /// expects a signal (see [`WorkerFacts::signal_expected`]), so `now`
    /// cannot change the result today and the board needs no timer. An
    /// ended session comes out as `SessionBoard::exited` at its end, the
    /// place the store gives a stored one (the tests hold the two equal).
    pub(crate) fn board(&self, now: DateTime<Utc>) -> SessionBoard {
        let facts = SessionFacts {
            worker: WorkerFacts {
                activity: self.activity,
                last_activity_at: self.last_activity_at,
                has_signal: self.seen,
                signal_expected: false,
                // Never archived while it publishes (the module docs).
                is_terminated: false,
            },
            interrupted: self.interrupted,
            has_artifact_output: false,
            policy: self.policy,
        };
        let pulls = match &self.pulls {
            Some(seen) => PullRequests::Seen(seen),
            None => PullRequests::Unseen,
        };
        let (column, display_status) = board::derive(&facts, pulls, now, board::NO_SIGNAL_GRACE);
        SessionBoard::new(column, display_status, self.activity, self.last_activity_at)
    }

    fn set(&mut self, activity: Activity, at: DateTime<Utc>) {
        self.activity = activity;
        self.touch(at);
    }

    fn touch(&mut self, at: DateTime<Utc>) {
        self.last_activity_at = at;
        self.seen = true;
    }
}

#[cfg(test)]
mod tests;
