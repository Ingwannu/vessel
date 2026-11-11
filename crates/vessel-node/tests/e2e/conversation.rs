//! Protocol 10 over the wire (milestone 11, track A): a session's
//! conversation as one call, live, from the store, and refused where there
//! is none; prompts and the queue; questions.

use std::collections::BTreeMap;

use vessel_proto::conversation::AnswerValue;
use vessel_proto::{
    AgentEvent, ContentBlock, Delivered, Delivery, ErrorCode, EventSeq, HarnessKind,
    QuestionAnswer, QuestionOutcome, QueueChange, QueueHold, SessionEvent, SessionInput,
    SessionStatus,
};
use vessel_rpc::CallError;

use crate::agents::{as_agent, credential, structured, vessel_mcp};
use crate::support::{
    Fixture, agent_of, ask, prompt, read_all, read_until, status_of, structured_spec,
    terminal_spec, within,
};

fn code<T: std::fmt::Debug>(result: Result<T, CallError>) -> ErrorCode {
    match result {
        Err(CallError::Remote(error)) => error.code,
        other => panic!("expected a refusal from the node, got {other:?}"),
    }
}

/// `session.conversation`: the agent's abilities and commands while it
/// runs; after the node's restart, what the store kept; a terminal
/// session has none.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_conversation_is_read_live_and_from_the_store() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        // The commands come when the agent sends them, before or after it
        // runs.
        read_until(&mut stream, &mut seen, |record| {
            matches!(record.event, SessionEvent::CommandsChanged { .. })
        })
        .await;
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        let live = client.conversation(id).await.unwrap();
        assert!(live.abilities.is_some_and(|abilities| abilities.steer));
        assert_eq!(live.commands.len(), 2);

        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let stored = client.conversation(id).await.unwrap();
        assert_eq!(stored.abilities, live.abilities, "kept by the store");
        assert!(stored.commands.is_empty(), "an ended session offers none");
        assert_eq!(
            code(client.conversation(vessel_proto::SessionId::random()).await),
            ErrorCode::NotFound
        );

        let terminal = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        assert_eq!(
            code(client.conversation(terminal).await),
            ErrorCode::Unsupported
        );
        node.stop().await.unwrap();
    })
    .await;
}

fn text(text: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: text.to_owned(),
    }]
}

/// `session.prompt` and `session.queue` over the wire: a prompt waits,
/// a held queue keeps it past a turn's end, `resume` sends it; refusals
/// are typed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompts_wait_in_the_nodes_queue() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        assert_eq!(
            client
                .prompt(id, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            Delivered::Started
        );
        assert_eq!(
            code(client.prompt(id, text("again"), Delivery::Start).await),
            ErrorCode::Conflict,
            "a turn runs"
        );
        let Delivered::Queued { entry } = client
            .prompt(id, text("plain"), Delivery::Queue)
            .await
            .unwrap()
        else {
            panic!("a turn runs: it waits");
        };
        let held = client.change_queue(id, QueueChange::Hold).await.unwrap();
        assert_eq!(held.queue.held, Some(QueueHold::Asked));
        assert_eq!(held.queue.entries[0].id, entry);
        assert_eq!(held.queue.entries[0].by, vessel_proto::Creator::Owner);
        assert_eq!(
            client
                .prompt(id, text("finish"), Delivery::Steer)
                .await
                .unwrap(),
            Delivered::Steered
        );
        let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { .. }
                }
            )
        })
        .await;
        let waiting = client.conversation(id).await.unwrap().queue;
        assert_eq!(waiting.entries.len(), 1, "held past the turn's end");
        let resumed = client.change_queue(id, QueueChange::Resume).await.unwrap();
        assert_eq!(resumed.delivered, Some(Delivered::Started));
        assert_eq!(
            code(client.change_queue(id, QueueChange::Remove { entry }).await),
            ErrorCode::NotFound,
            "sent"
        );
        assert_eq!(
            code(client.prompt(id, Vec::new(), Delivery::Queue).await),
            ErrorCode::InvalidRequest
        );

        let terminal = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        assert_eq!(
            code(client.prompt(terminal, text("x"), Delivery::Queue).await),
            ErrorCode::Unsupported
        );
        node.stop().await.unwrap();

        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        assert_eq!(
            code(client.change_queue(id, QueueChange::Clear).await),
            ErrorCode::Conflict,
            "ended: its queue comes back with session.resume"
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// An owner's session takes questions: the agent asks, the owner answers
/// with `session.send`'s `answer_question`, and the log keeps the outcome.
/// A session an agent created runs unwatched and is never offered them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_owners_session_asks_and_an_agents_does_not() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut parent = structured(&owner, &fixture).await;
        // Its agent's credential first: reading it takes a turn of its own.
        let (_program, socket, auth) = credential(&owner, &mut parent).await;

        owner.send(parent.id, prompt("ask")).await.unwrap();
        let mut stream = owner.follow(parent.id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::QuestionAsked(_)))
        })
        .await;
        // The stream starts at the log's start: the credential's turn too.
        let asked_at = seen.len();
        let waiting = owner.conversation(parent.id).await.unwrap().questions;
        let [question] = waiting.as_slice() else {
            panic!("one question waits: {waiting:?}");
        };
        let values = BTreeMap::from([(
            "way".parse().unwrap(),
            AnswerValue::Choice("left".to_owned()),
        )]);
        let answer = |answer| SessionInput::AnswerQuestion {
            id: question.id,
            answer,
        };
        owner
            .send(
                parent.id,
                answer(QuestionAnswer::Accept {
                    values: values.clone(),
                }),
            )
            .await
            .unwrap();
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        let said: String = seen[asked_at..]
            .iter()
            .filter_map(|record| match agent_of(record) {
                Some(AgentEvent::MessageChunk {
                    content: ContentBlock::Text { text },
                }) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(said, r#"answered: accept {"way":"left"}"#);
        assert_eq!(
            code(owner.send(parent.id, answer(QuestionAnswer::Decline)).await),
            ErrorCode::Conflict,
            "the first answer won"
        );
        let logged = read_all(&owner, parent.id).await;
        assert!(
            logged.iter().any(|record| matches!(
                agent_of(record),
                Some(AgentEvent::QuestionResolved { id, outcome: QuestionOutcome::Answered { values: given } })
                    if *id == question.id && *given == values
            )),
            "the outcome is in the log"
        );

        // An agent's own session is not offered questions (review item 4).
        let agent = as_agent(&socket, &auth).await;
        let child = agent
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        let mut stream = owner.follow(child, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        assert_eq!(
            ask(&owner, &mut stream, &mut seen, child, "ask").await,
            "no questions"
        );
        node.stop().await.unwrap();
    })
    .await;
}
