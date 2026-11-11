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
//!   makes it active; a permission request makes it `waiting_input` until
//!   every request open is resolved, when the turn goes on (active). The
//!   latest turn is interrupted when it ended `cancelled`, until the next
//!   one starts. Terminal: output makes it active, quiet makes it idle. Any
//!   final status makes it `exited` and the session terminated.
//! - Last activity: the time of the record that showed the agent doing
//!   something (the handshake done, a turn's start or end, a permission
//!   asked or answered, terminal output, the end). Message and tool-call
//!   chunks inside a turn do not move it: the contract lists what does, and
//!   a turn's start already says the agent is at work. A terminal's quiet
//!   does not move it either: quiet is the absence of activity.
//! - Must stay true: a record changes the facts the same way whichever
//!   actor published it, so a replay of the log gives the same facts (the
//!   quiet timer aside).

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use vessel_proto::{Activity, AgentEvent, PermissionId, SessionBoard, SessionEvent, StopReason};

use crate::board::{self, Policy, PullRequests, SessionFacts, WorkerFacts};

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
    /// Permission requests asked and not yet resolved. Rarely more than
    /// one; a set so a resolution matches its own request, whatever order
    /// several are answered in.
    pending: HashSet<PermissionId>,
    terminated: bool,
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
            terminated: false,
        }
    }

    /// Read one record the session is publishing, stamped `at`.
    pub(crate) fn observe(&mut self, event: &SessionEvent, at: DateTime<Utc>) {
        match event {
            SessionEvent::Status { status } => {
                // `Starting` is where a tracker begins and `Running` says
                // nothing about turns; only the end changes the facts.
                if !status.is_live() {
                    self.pending.clear();
                    self.terminated = true;
                    self.set(Activity::Exited, at);
                }
            }
            SessionEvent::Terminal(_) => self.set(Activity::Active, at),
            SessionEvent::Agent { event } => self.agent(event, at),
            SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. } => {}
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
                let _new = self.pending.insert(request.id);
                self.set(Activity::WaitingInput, at);
            }
            AgentEvent::PermissionResolved { id, .. } => {
                // A resolution after the turn ended (the agent's driver
                // withdraws what a closed turn left open) changes nothing:
                // not the activity, not when the agent was last seen.
                if !self.pending.remove(id) {
                    return;
                }
                // The turn goes on once nothing waits on the owner.
                if self.pending.is_empty() && self.activity == Activity::WaitingInput {
                    self.set(Activity::Active, at);
                } else {
                    self.touch(at);
                }
            }
            AgentEvent::TurnEnded { reason } => {
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
            AgentEvent::UserMessageChunk { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. }
            | AgentEvent::HarnessLog { .. } => {}
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
    /// No Vessel node can see pull requests yet and no session has a fleet
    /// policy or an artifact, so those facts are fixed here; and nothing
    /// expects a signal (see [`WorkerFacts::signal_expected`]), so `now`
    /// cannot change the result today and the board needs no timer.
    pub(crate) fn board(&self, now: DateTime<Utc>) -> SessionBoard {
        // An ended session's place has one definition, shared with the
        // store's (`SessionBoard::archived`); the reducer agrees (its
        // tests hold `derive` to the archive for every ended session).
        if self.terminated {
            return SessionBoard::archived(self.last_activity_at);
        }
        let facts = SessionFacts {
            worker: WorkerFacts {
                activity: self.activity,
                last_activity_at: self.last_activity_at,
                has_signal: self.seen,
                signal_expected: false,
                is_terminated: self.terminated,
            },
            interrupted: self.interrupted,
            has_artifact_output: false,
            policy: Policy::default(),
        };
        let (column, display_status) =
            board::derive(&facts, PullRequests::Unseen, now, board::NO_SIGNAL_GRACE);
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
mod tests {
    use vessel_proto::{
        DisplayStatus, KanbanColumn, PermissionOutcome, PermissionRequest, SessionStatus,
        TerminalBytes, TerminalOutput, ToolCallId,
    };

    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap()
    }

    fn agent(event: AgentEvent) -> SessionEvent {
        SessionEvent::Agent { event }
    }

    fn status(status: SessionStatus) -> SessionEvent {
        SessionEvent::Status { status }
    }

    fn requested(id: PermissionId) -> SessionEvent {
        agent(AgentEvent::PermissionRequested(PermissionRequest {
            id,
            tool_call: ToolCallId::new("call-1"),
            options: Vec::new(),
        }))
    }

    fn resolved(id: PermissionId) -> SessionEvent {
        agent(AgentEvent::PermissionResolved {
            id,
            outcome: PermissionOutcome::Cancelled,
        })
    }

    fn ended(reason: StopReason) -> SessionEvent {
        agent(AgentEvent::TurnEnded { reason })
    }

    fn started() -> SessionEvent {
        agent(AgentEvent::TurnStarted {
            content: Vec::new(),
        })
    }

    /// Feed `events`, the n-th stamped n seconds after the epoch, and
    /// return the board after each.
    fn boards(events: &[SessionEvent]) -> Vec<(KanbanColumn, DisplayStatus, Activity, i64)> {
        let mut tracker = Tracker::new(at(0));
        events
            .iter()
            .zip(1..)
            .map(|(event, seconds)| {
                tracker.observe(event, at(seconds));
                let board = tracker.board(at(seconds));
                (
                    board.column,
                    board.display_status,
                    board.activity,
                    board.last_activity_at.timestamp(),
                )
            })
            .collect()
    }

    #[test]
    fn a_structured_session_through_a_turn_with_two_permissions() {
        let first = PermissionId::random();
        let second = PermissionId::random();
        let building = KanbanColumn::Building;
        assert_eq!(
            boards(&[
                status(SessionStatus::Starting),
                agent(AgentEvent::Ready {
                    harness_session: vessel_proto::HarnessSessionId::new("s"),
                }),
                status(SessionStatus::Running),
                started(),
                agent(AgentEvent::MessageChunk {
                    content: vessel_proto::ContentBlock::Text {
                        text: "hi".to_owned()
                    },
                }),
                requested(first),
                requested(second),
                resolved(second),
                resolved(first),
                ended(StopReason::EndTurn),
            ]),
            [
                (building, DisplayStatus::Working, Activity::Active, 0),
                (building, DisplayStatus::Idle, Activity::Idle, 2),
                (building, DisplayStatus::Idle, Activity::Idle, 2),
                (building, DisplayStatus::Working, Activity::Active, 4),
                // A chunk inside the turn is not a new activity.
                (building, DisplayStatus::Working, Activity::Active, 4),
                (building, DisplayStatus::Blocked, Activity::WaitingInput, 6),
                (building, DisplayStatus::Blocked, Activity::WaitingInput, 7),
                // One still open: still waiting on the owner.
                (building, DisplayStatus::Blocked, Activity::WaitingInput, 8),
                (building, DisplayStatus::Working, Activity::Active, 9),
                (building, DisplayStatus::Idle, Activity::Idle, 10),
            ]
        );
    }

    #[test]
    fn a_cancelled_turn_is_interrupted_until_the_next_one() {
        let phrases: Vec<DisplayStatus> = boards(&[
            started(),
            ended(StopReason::Cancelled),
            started(),
            ended(StopReason::EndTurn),
            started(),
            agent(AgentEvent::TurnFailed {
                message: "no".to_owned(),
            }),
        ])
        .into_iter()
        .map(|(_, phrase, _, _)| phrase)
        .collect();
        assert_eq!(
            phrases,
            [
                DisplayStatus::Working,
                DisplayStatus::Interrupted,
                DisplayStatus::Working,
                DisplayStatus::Idle,
                DisplayStatus::Working,
                DisplayStatus::Idle,
            ]
        );
    }

    /// A turn ends with its requests: the agent's driver withdraws what is
    /// still open, sometimes after the turn's end, and that must not make
    /// an idle agent look busy again.
    #[test]
    fn requests_end_with_their_turn() {
        let id = PermissionId::random();
        let last = boards(&[
            started(),
            requested(id),
            ended(StopReason::Cancelled),
            resolved(id),
        ]);
        // The withdrawal of a request the turn's end already closed is no
        // activity: the agent was last seen ending the turn (3), not 4.
        assert_eq!(
            last.last(),
            Some(&(
                KanbanColumn::Building,
                DisplayStatus::Interrupted,
                Activity::Idle,
                3
            ))
        );
    }

    #[test]
    fn a_terminal_is_active_on_output_and_idle_when_quiet() {
        let output = SessionEvent::Terminal(TerminalOutput {
            data: TerminalBytes::new(bytes::Bytes::from_static(b"x")),
        });
        let mut tracker = Tracker::new(at(0));
        tracker.observe(&status(SessionStatus::Starting), at(0));
        tracker.observe(&status(SessionStatus::Running), at(0));
        assert_eq!(tracker.board(at(0)).display_status, DisplayStatus::Working);
        tracker.quiet();
        let quiet = tracker.board(at(6));
        assert_eq!(
            (quiet.display_status, quiet.activity),
            (DisplayStatus::Idle, Activity::Idle)
        );
        assert_eq!(quiet.last_activity_at, at(0), "quiet is not activity");
        tracker.observe(&output, at(7));
        let busy = tracker.board(at(7));
        assert_eq!(
            (busy.display_status, busy.activity, busy.last_activity_at),
            (DisplayStatus::Working, Activity::Active, at(7))
        );
    }

    #[test]
    fn every_end_is_the_archive() {
        for end in [
            SessionStatus::Exited { code: Some(0) },
            SessionStatus::Failed {
                message: "no".to_owned(),
            },
            SessionStatus::Interrupted,
        ] {
            let id = PermissionId::random();
            let mut tracker = Tracker::new(at(0));
            tracker.observe(&started(), at(1));
            tracker.observe(&requested(id), at(2));
            tracker.observe(&status(end.clone()), at(3));
            assert_eq!(
                tracker.board(at(9)),
                SessionBoard::new(
                    KanbanColumn::Archive,
                    DisplayStatus::Terminated,
                    Activity::Exited,
                    at(3)
                ),
                "{end:?}"
            );
            // A quiet timer that fires late cannot revive it.
            tracker.quiet();
            assert_eq!(tracker.board(at(9)).activity, Activity::Exited);
        }
    }
}
