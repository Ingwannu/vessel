//! Tests of a session's activity and its place on the board.

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
        plan_proposal: None,
        delivery_state: vessel_proto::timeline::PermissionDeliveryState::Pending,
        tool_scope: None,
        id,
        tool_call: Some(ToolCallId::new("call-1")),
        options: Vec::new(),

        presentation: None,
    }))
}

fn resolved(id: PermissionId) -> SessionEvent {
    agent(AgentEvent::PermissionResolved {
        id,
        outcome: PermissionOutcome::Cancelled,
    })
}

fn ended(reason: StopReason) -> SessionEvent {
    agent(AgentEvent::TurnEnded {
        reason,
        usage: None,
        point: None,
    })
}

fn started() -> SessionEvent {
    agent(AgentEvent::TurnStarted {
        provenance: None,
        content: Vec::new(),

        client_message_id: None,
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
                abilities: None
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

/// ADR 0026: an ended session is in building as `exited`, its end its
/// last activity, whatever it was doing and however it ended: the
/// place the store gives it (`SessionBoard::exited`).
#[test]
fn every_end_is_exited_in_building() {
    for end in [
        SessionStatus::Exited { code: Some(0) },
        SessionStatus::failed("no"),
        SessionStatus::Interrupted,
    ] {
        let id = PermissionId::random();
        let mut tracker = Tracker::new(at(0));
        tracker.observe(&started(), at(1));
        tracker.observe(&requested(id), at(2));
        tracker.observe(&status(end.clone()), at(3));
        assert_eq!(tracker.board(at(9)), SessionBoard::exited(at(3)), "{end:?}");
        // A quiet timer that fires late cannot revive it.
        tracker.quiet();
        assert_eq!(tracker.board(at(9)).activity, Activity::Exited);
    }
    // Nor does a stop mid-turn speak once the agent is gone.
    let mut tracker = Tracker::new(at(0));
    tracker.observe(&started(), at(1));
    tracker.observe(&ended(StopReason::Cancelled), at(2));
    tracker.observe(&status(SessionStatus::Interrupted), at(3));
    assert_eq!(tracker.board(at(9)), SessionBoard::exited(at(3)));
}

/// The node's view of a session's pull request: `seen` its facts (`None`
/// unseen), `feedback` its feedback switch.
fn view(seen: Option<Vec<PullRequestFacts>>, feedback: bool) -> ForgeView {
    use vessel_proto::forge::{ForgeAccess, SessionPullRequest};
    ForgeView {
        info: SessionPullRequest {
            access: ForgeAccess::Ready { login: None },
            summary: None,
            feedback,
        },
        seen: seen.map(Arc::from),
        policy: Policy {
            auto_review: false,
            auto_inject_review: feedback,
            auto_inject_ci: feedback,
        },
    }
}

/// Protocol 11 (ADR 0029): the node's pull request places the session
/// through the same reducer; its feedback switch is the fleet policy, so
/// an agent being sent its failing checks is fixing them in validating;
/// and the end drops both, so an ended session is placed as a stored one.
#[test]
fn a_pull_request_places_the_session_until_it_ends() {
    let place = |tracker: &Tracker| {
        let board = tracker.board(at(9));
        (board.column, board.display_status)
    };
    let mut tracker = Tracker::new(at(0));
    tracker.observe(&started(), at(1));
    tracker.observe(&ended(StopReason::EndTurn), at(2));
    assert_eq!(
        place(&tracker),
        (KanbanColumn::Building, DisplayStatus::Idle)
    );

    tracker.forge(Some(&view(Some(Vec::new()), false)));
    assert_eq!(
        place(&tracker),
        (KanbanColumn::Building, DisplayStatus::AwaitingPr),
        "watched, none yet"
    );

    let failing = PullRequestFacts {
        ci: board::CiState::Failing,
        ..PullRequestFacts::new(
            board::PullRequestUrl::new("https://github.com/o/n/pull/1"),
            at(0),
        )
    };
    tracker.forge(Some(&view(Some(vec![failing.clone()]), false)));
    assert_eq!(
        place(&tracker),
        (KanbanColumn::NeedsReview, DisplayStatus::CiFailing)
    );

    tracker.forge(Some(&view(Some(vec![failing]), true)));
    tracker.observe(&started(), at(3));
    assert_eq!(
        place(&tracker),
        (KanbanColumn::Validating, DisplayStatus::FixingCi)
    );

    tracker.forge(None);
    assert_eq!(
        place(&tracker),
        (KanbanColumn::Building, DisplayStatus::Working),
        "no longer watched: placed as before"
    );

    tracker.forge(Some(&view(Some(Vec::new()), true)));
    tracker.observe(&status(SessionStatus::Exited { code: Some(0) }), at(4));
    assert_eq!(tracker.board(at(9)), SessionBoard::exited(at(4)));
}
