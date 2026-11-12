//! Questions (protocol 10) in the session runtime, with `fake-acp` as
//! Claude's adapter: a question waits in the conversation and on the
//! board until a client answers it, and the refusals are typed.
//!
//! Tests wait on records, never on time; the timeout only turns a hang
//! into a failure.

use vessel_proto::{
    Activity, AgentEvent, ContentBlock, ErrorCode, QuestionId, QuestionOutcome, SessionEvent,
    StopReason,
};
use vessel_session::LaunchOptions;

use asking::{answer, board};
use support::{Fixture, Log, prompt, structured, within};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "the shared runtime fixture, not every helper of it"
)]
mod support;

#[cfg(test)]
mod asking {
    use std::collections::BTreeMap;

    use vessel_proto::conversation::AnswerValue;
    use vessel_proto::{Activity, QuestionAnswer, QuestionId, SessionInfo, SessionInput};
    use vessel_session::SessionHandle;

    fn activity(info: &SessionInfo) -> Option<Activity> {
        info.board.as_ref().map(|board| board.activity)
    }

    /// Wait until the board shows `wanted`, or no longer does.
    pub(super) async fn board(handle: &SessionHandle, wanted: Activity, is: bool) {
        let mut info = handle.watch_info();
        let _seen = info
            .wait_for(|info| (activity(info) == Some(wanted)) == is)
            .await
            .unwrap();
    }

    /// An accepting answer choosing `values`.
    pub(super) fn answer(question: QuestionId, values: &[(&str, &str)]) -> SessionInput {
        SessionInput::AnswerQuestion {
            id: question,
            answer: QuestionAnswer::Accept {
                values: values
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.parse().unwrap(),
                            AnswerValue::Choice((*value).to_owned()),
                        )
                    })
                    .collect::<BTreeMap<_, _>>(),
            },
        }
    }
}

/// A session that takes questions: the agent asks, the conversation and
/// the board show it waiting, an answer settles it once.
#[tokio::test]
async fn a_question_waits_until_it_is_answered() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture
            .start_with(
                structured(Vec::new()),
                LaunchOptions {
                    questions: true,
                    ..LaunchOptions::default()
                },
            )
            .await;
        let mut log = Log::of(&handle);
        handle.send(prompt("ask")).await.unwrap();
        log.until(|event| {
            matches!(
                event,
                SessionEvent::Agent {
                    event: AgentEvent::QuestionAsked(_)
                }
            )
        })
        .await;
        let waiting = handle.conversation().unwrap().questions;
        let [question] = waiting.as_slice() else {
            panic!("one question waits: {waiting:?}");
        };
        board(&handle, Activity::WaitingInput, true).await;

        let refused = |input| {
            let handle = handle.clone();
            async move { handle.send(input).await.unwrap_err().code() }
        };
        assert_eq!(
            refused(answer(question.id, &[("way", "up")])).await,
            ErrorCode::InvalidRequest,
            "not one of the options"
        );
        assert_eq!(
            refused(answer(QuestionId::random(), &[])).await,
            ErrorCode::NotFound
        );
        assert_eq!(
            handle.conversation().unwrap().questions.len(),
            1,
            "still waits"
        );

        handle
            .send(answer(question.id, &[("way", "right")]))
            .await
            .unwrap();
        let from = log.records.len();
        log.since(from, |event| {
            matches!(
                event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded {
                        reason: StopReason::EndTurn,
                        ..
                    }
                }
            )
        })
        .await;
        let outcomes: Vec<&QuestionOutcome> = log
            .records
            .iter()
            .filter_map(|record| {
                if let SessionEvent::Agent {
                    event: AgentEvent::QuestionResolved { outcome, .. },
                } = &record.event
                {
                    Some(outcome)
                } else {
                    None
                }
            })
            .collect();
        assert!(
            matches!(outcomes.as_slice(), [QuestionOutcome::Answered { .. }]),
            "{outcomes:?}"
        );
        assert!(handle.conversation().unwrap().questions.is_empty());
        board(&handle, Activity::WaitingInput, false).await;
        assert_eq!(
            refused(answer(question.id, &[("way", "left")])).await,
            ErrorCode::Conflict,
            "the first answer won"
        );
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}

/// The runtime's default: the agent is not offered questions, so it does
/// not ask.
#[tokio::test]
async fn a_session_without_questions_is_not_asked() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("ask")).await.unwrap();
        log.turn_over().await;
        let said: Vec<&str> = log
            .records
            .iter()
            .filter_map(|record| {
                if let SessionEvent::Agent {
                    event:
                        AgentEvent::MessageChunk {
                            content: ContentBlock::Text { text },
                        },
                } = &record.event
                {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(said, ["no questions"]);
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}
