//! Protocol 10's wire (Track A, the conversation): the calls' versions,
//! and what an older connection receives in place of each new event.

use std::collections::BTreeMap;

use chrono::DateTime;

use super::*;
use crate::agent::{AgentEvent, ContentBlock};
use crate::conversation::{
    AgentCommand, AnswerValue, AttachmentKind, AttachmentRef, ContextUsage, Delivery, PromptQueue,
    PromptSession, Question, QuestionAnswer, QuestionOutcome, QueueHold, QueuedPrompt,
};
use crate::event::{EventRecord, SessionEvent};
use crate::id::{AttachmentId, EventSeq, QuestionId, QueueEntryId, SessionId};
use crate::model::TokenCount;
use crate::profile::Creator;
use crate::session::SessionInput;

const V9: ProtocolVersion = ProtocolVersion::new(9);
const V10: ProtocolVersion = ProtocolVersion::new(10);

/// What a stand-in log line looks like, as `for_protocol` returns it.
#[expect(
    clippy::unnecessary_wraps,
    reason = "compared with `for_protocol`'s `Option` as it is"
)]
fn log(line: &str) -> Option<SessionEvent> {
    Some(SessionEvent::Agent {
        event: AgentEvent::HarnessLog {
            line: line.to_owned(),
        },
    })
}

fn attachment() -> AttachmentRef {
    AttachmentRef {
        id: AttachmentId::random(),
        name: "shot.png".parse().unwrap(),
        kind: AttachmentKind::Image,
        mime: "image/png".parse().unwrap(),
        size: 3,
    }
}

fn text(text: &str) -> ContentBlock {
    ContentBlock::Text {
        text: text.to_owned(),
    }
}

#[test]
fn the_new_calls_need_protocol_ten() {
    let session = SessionId::random();
    let prompt = PromptSession {
        session,
        content: vec![text("hi")],
        delivery: Delivery::Queue,
        notify: false,
    };
    let told = PromptSession {
        notify: true,
        ..prompt.clone()
    };
    assert_eq!(told.into_call().since(), Some(ProtocolVersion::new(11)));
    let prompt = prompt.into_call();
    assert_eq!(prompt.since(), Some(V10));
    // An answer to a question, or a prompt naming an attachment, on the
    // old call: an older node could not decode either.
    let answer = SendInput {
        session,
        input: SessionInput::AnswerQuestion {
            id: QuestionId::random(),
            answer: QuestionAnswer::Decline,
        },
    }
    .into_call();
    assert_eq!(answer.since(), Some(V10));
    let attached = SendInput {
        session,
        input: SessionInput::Prompt {
            content: vec![
                text("look"),
                ContentBlock::Attachment {
                    attachment: attachment(),
                },
            ],
        },
    }
    .into_call();
    assert_eq!(attached.since(), Some(V10));
    let plain = SendInput {
        session,
        input: SessionInput::Prompt {
            content: vec![text("hi")],
        },
    }
    .into_call();
    assert_eq!(plain.since(), Some(ProtocolVersion::new(2)));
    for method in [
        Method::PromptSession,
        Method::ChangeQueue,
        Method::GetConversation,
        Method::ResumeSession,
        Method::MarkSession,
        Method::DeleteSession,
        Method::BeginUpload,
        Method::WriteUpload,
        Method::FinishUpload,
        Method::ReadAttachment,
    ] {
        assert_eq!(method.since(), Some(V10), "{method:?}");
        assert!(method.forwardable(), "{method:?}");
    }
    assert_eq!(
        Method::GetConversation
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Read
    );
    assert_eq!(
        Method::ReadAttachment
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Read
    );
    assert_eq!(
        Method::DeleteSession
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Operate
    );
}

#[test]
fn session_events_of_ten_reach_older_clients_as_log_lines() {
    let entry = QueuedPrompt {
        id: QueueEntryId::random(),
        content: vec![text("next")],
        queued_at: DateTime::UNIX_EPOCH,
        revision: 0,
        by: Creator::Owner,
    };
    let held = SessionEvent::QueueChanged {
        queue: PromptQueue {
            entries: vec![entry.clone(), entry],
            held: Some(QueueHold::Cancelled),
        },
    };
    assert_eq!(held.required_protocol(), Some(V10));
    assert_eq!(held.for_protocol(V10), None);
    assert_eq!(
        held.for_protocol(V9),
        log("vessel: queue: 2 waiting (held: cancelled)")
    );

    let commands = (0..12)
        .map(|n| AgentCommand {
            name: format!("c{n}").parse().unwrap(),
            description: String::new(),
            hint: None,
            kind: None,
        })
        .collect();
    assert_eq!(
        SessionEvent::CommandsChanged { commands }.for_protocol(V9),
        log("vessel: commands: /c0, /c1, /c2, /c3, /c4, /c5, /c6, /c7, /c8, /c9 and 2 more")
    );
    assert_eq!(
        SessionEvent::CommandsChanged {
            commands: Vec::new()
        }
        .for_protocol(V9),
        log("vessel: commands: none")
    );

    let usage = SessionEvent::UsageChanged {
        usage: ContextUsage {
            observation: crate::conversation::UsageObservation::Current,
            used: TokenCount::new(41_200),
            size: TokenCount::new(200_000),
            cost: None,
        },
    };
    assert_eq!(
        usage.for_protocol(V9),
        log("vessel: context: 41200 of 200000 tokens")
    );
}

#[test]
fn agent_events_of_ten_reach_older_clients_too() {
    let agent = |event| SessionEvent::Agent { event };
    let steered = agent(AgentEvent::Steered {
        content: vec![text("\n  also run the tests\nplease")],
    });
    assert_eq!(steered.required_protocol(), Some(V10));
    assert_eq!(
        steered.for_protocol(V9),
        log("vessel: steered: also run the tests")
    );
    let asked = agent(AgentEvent::QuestionAsked(Question {
        id: QuestionId::random(),
        message: "Which one?\nPick one".to_owned(),
        tool_call: None,
        fields: Vec::new(),
    }));
    assert_eq!(
        asked.for_protocol(V9),
        log("vessel: the agent asks: Which one?")
    );
    let answered = agent(AgentEvent::QuestionResolved {
        id: QuestionId::random(),
        outcome: QuestionOutcome::Answered {
            values: BTreeMap::from([("q".parse().unwrap(), AnswerValue::Boolean(true))]),
        },
    });
    assert_eq!(answered.for_protocol(V9), log("vessel: question answered"));

    // A prompt that names an attachment stays a prompt: the attachment
    // becomes a link an older client can decode.
    let image = attachment();
    let id = image.id;
    let started = agent(AgentEvent::TurnStarted {
        content: vec![text("look"), ContentBlock::Attachment { attachment: image }],
    });
    assert_eq!(started.required_protocol(), Some(V10));
    assert_eq!(
        started.for_protocol(V9),
        Some(agent(AgentEvent::TurnStarted {
            content: vec![
                text("look"),
                ContentBlock::ResourceLink {
                    uri: format!("attachment:{id}"),
                    name: "shot.png".to_owned(),
                },
            ],
        }))
    );
    let plain = agent(AgentEvent::TurnStarted {
        content: vec![text("look")],
    });
    assert_eq!(plain.required_protocol(), Some(ProtocolVersion::new(2)));
    assert_eq!(plain.for_protocol(V9), None);
}

#[test]
fn a_page_with_a_new_event_is_rewritten_for_older_clients() {
    let record = |seq, event| EventRecord {
        seq: EventSeq::new(seq),
        at: DateTime::UNIX_EPOCH,
        event,
    };
    let page = EventPage {
        records: vec![
            record(
                0,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded {
                        reason: crate::agent::StopReason::EndTurn,
                        usage: None,
                        point: None,
                    },
                },
            ),
            record(
                1,
                SessionEvent::QueueChanged {
                    queue: PromptQueue::default(),
                },
            ),
        ],
        next: EventSeq::new(2),
        at_end: true,
    };
    assert_eq!(page.for_protocol(V10), None);
    let older = page.for_protocol(V9).unwrap();
    assert_eq!(older.records[0], page.records[0]);
    assert_eq!(older.records[1].seq, EventSeq::new(1));
    assert_eq!(
        Some(older.records[1].event.clone()),
        log("vessel: queue: 0 waiting")
    );
}
