//! The main composer can show a full side session too. All of its text
//! must take the side ingress; the normal queue and implicit-resume path
//! would be refused by the node. A refused route leaves the draft intact.

use vessel_proto::automation::{InvalidAutomationText, PromptText};
use vessel_proto::{Activity, SessionId, SessionInfo, SessionMode, SessionStatus};

use super::{SideCommand, command};
use crate::timeline::Timeline;

/// What the intercept slot does before the normal composer's submit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submission {
    /// Empty draft: keep it as-is and make no call.
    Noop,
    /// Ordinary parent input; let its existing composer handle it.
    Normal,
    /// Show the parent's side panel, optionally asking its first question.
    Open(SideCommand),
    /// Start a question in an already shown full side conversation.
    Ask {
        /// The side session, never its parent's id.
        side: SessionId,
        /// A bounded question; no queue or steer choice exists here.
        question: PromptText,
    },
    /// Keep the draft and explain why no call was made.
    Hold(SubmitProblem),
}

/// Local refusal states; their Display is diagnostic, the panel's words
/// are translated separately.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubmitProblem {
    /// Session metadata or handshake has not arrived.
    #[error("the conversation is still loading")]
    Loading,
    /// The side is working or waiting on a tool/question.
    #[error("the side conversation is busy")]
    Busy,
    /// Ended side sessions need the person's explicit resume action.
    #[error("resume the side conversation before asking")]
    Resume,
    /// The same no-nested rule as the server and GOAT.
    #[error("open a side question from the original conversation")]
    Nested,
    /// The parent runs in the agent's terminal.
    #[error("a terminal session cannot open a side conversation")]
    Terminal,
    /// Native commands cannot carry this feature's refreshed context.
    #[error("run native commands in an ordinary conversation")]
    NativeCommand,
    /// The wire's bounded-text parse failed; never show its raw string.
    #[error("the side question is invalid")]
    Question(#[source] InvalidAutomationText),
}

/// Select a route without clearing the composer or sending anything.
/// Missing metadata holds input because a card alone does not reveal a
/// side's parent link. Never guess and send through ordinary ingress.
#[must_use]
pub fn submission(
    session: Option<&SessionInfo>,
    timeline: Option<&Timeline>,
    text: &str,
) -> Submission {
    if text.trim_matches(crate::timeline::is_space).is_empty() {
        return Submission::Noop;
    }
    let Some(session) = session else {
        return Submission::Hold(SubmitProblem::Loading);
    };
    if session.spec.side_question_of.is_some() {
        return side_submission(session, timeline, text);
    }
    match command(text) {
        None => Submission::Normal,
        Some(Err(error)) => Submission::Hold(SubmitProblem::Question(error)),
        Some(Ok(command)) => match session.spec.mode {
            SessionMode::Terminal { .. } => Submission::Hold(SubmitProblem::Terminal),
            SessionMode::Structured { .. } => Submission::Open(command),
        },
    }
}

fn side_submission(session: &SessionInfo, timeline: Option<&Timeline>, text: &str) -> Submission {
    if command(text).is_some() {
        return Submission::Hold(SubmitProblem::Nested);
    }
    if text
        .trim_start_matches(crate::timeline::is_space)
        .starts_with('/')
    {
        return Submission::Hold(SubmitProblem::NativeCommand);
    }
    // A list/info answer can precede Ready or a new turn. Use the stream
    // only once it caught up to that answer, so partial replay cannot
    // override newer metadata with an old Running state.
    let current = timeline
        .filter(|timeline| timeline.next() >= session.next_seq && timeline.status().is_some());
    let status = current
        .and_then(Timeline::status)
        .unwrap_or(&session.status);
    match status {
        SessionStatus::Starting => return Submission::Hold(SubmitProblem::Loading),
        SessionStatus::Exited { .. }
        | SessionStatus::Failed { .. }
        | SessionStatus::Interrupted => {
            return Submission::Hold(SubmitProblem::Resume);
        }
        SessionStatus::Running => {}
    }
    let busy = current.map_or_else(
        || {
            session.board.as_ref().is_some_and(|board| {
                matches!(board.activity, Activity::Active | Activity::WaitingInput)
            })
        },
        |timeline| {
            timeline.in_turn()
                || timeline.pending().is_some()
                || !timeline.waiting_questions().is_empty()
        },
    );
    if busy {
        return Submission::Hold(SubmitProblem::Busy);
    }
    match text.parse() {
        Ok(question) => Submission::Ask {
            side: session.id,
            question,
        },
        Err(error) => Submission::Hold(SubmitProblem::Question(error)),
    }
}
