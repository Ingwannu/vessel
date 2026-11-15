//! An agent's plan as it changes during a turn, in the shape Claude Code's
//! adapter sends it (`claude-agent-acp` turns each `TodoWrite` into an ACP
//! `plan` update holding the whole list), for the native app's plan rows
//! to be checked against (track C, Paseo's task list):
//!
//! `plans`: four steps created, all to do (one Korean, one long); the same
//! list sent again unchanged; the first started; the first done as the
//! second starts (one update, two changes); a fifth step added once work
//! began; the second done as the third starts. Then one message chunk,
//! `plans done`.

use PlanEntryStatus::{Completed, InProgress, Pending};
use agent_client_protocol::Error;
use agent_client_protocol::v1::schema::{
    ContentChunk, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, SessionId,
    SessionNotification, SessionUpdate, StopReason,
};
use tokio_util::sync::CancellationToken;

/// The steps, in order.
const STEPS: [&str; 5] = [
    "Read the fold and find where a batch ends inside a message",
    "긴 대화에서 스크롤이 끊기지 않는지 확인",
    "Write the regression test that replays a recorded turn split at every record and compares the folds",
    "Run the whole gate",
    "Say what changed",
];

/// The updates, each the whole list: how many steps, and each one's state.
const UPDATES: [&[PlanEntryStatus]; 6] = [
    &[Pending, Pending, Pending, Pending],
    &[Pending, Pending, Pending, Pending],
    &[InProgress, Pending, Pending, Pending],
    &[Completed, InProgress, Pending, Pending],
    &[Completed, InProgress, Pending, Pending, Pending],
    &[Completed, Completed, InProgress, Pending, Pending],
];

/// The plan with `statuses`, one per step from the first.
fn plan(statuses: &[PlanEntryStatus]) -> Plan {
    Plan::new(
        STEPS
            .iter()
            .zip(statuses)
            .map(|(step, status)| PlanEntry::new(*step, PlanEntryPriority::Medium, status.clone()))
            .collect(),
    )
}

/// Run the `plans` turn.
pub(crate) fn turn(
    cx: &agent_client_protocol::v1::ConnectionTo<agent_client_protocol::Client>,
    session: &SessionId,
    turn: &CancellationToken,
) -> Result<StopReason, Error> {
    let say = |update| cx.send_notification(SessionNotification::new(session.clone(), update));
    for statuses in UPDATES {
        if turn.is_cancelled() {
            return Ok(StopReason::Cancelled);
        }
        say(SessionUpdate::Plan(plan(statuses)))?;
    }
    say(SessionUpdate::AgentMessageChunk(ContentChunk::new(
        super::text("plans done"),
    )))?;
    Ok(StopReason::EndTurn)
}
