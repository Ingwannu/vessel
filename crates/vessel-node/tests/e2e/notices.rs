//! The starter told through a real node (protocol 11, ADR 0033): an agent
//! that prompts its child with `notify` hears, as a prompt queued into its
//! own session, when that turn finishes, when it waits for an answer, when
//! it fails, and when the child ends before a queued prompt started a
//! turn; nobody else may ask for it.

use vessel_client::{NodeClient, SessionStream};
use vessel_proto::{
    AgentEvent, ContentBlock, Delivered, Delivery, ErrorCode, EventRecord, EventSeq, SessionEvent,
    SessionId,
};

use crate::agents::{as_agent, code, credential, following, structured, vessel_mcp};
use crate::support::{Fixture, read_until, structured_spec, within};

fn text(prompt: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: prompt.to_owned(),
    }]
}

/// Whether `record` carries a prompt (a turn's or a queue entry's) with
/// every one of `words` in it.
fn says(record: &EventRecord, words: &[&str]) -> bool {
    let has = |content: &[ContentBlock]| {
        content.iter().any(|block| {
            matches!(block, ContentBlock::Text { text }
                if words.iter().all(|word| text.contains(word)))
        })
    };
    if let SessionEvent::Agent {
        event:
            AgentEvent::TurnStarted {
                provenance: None,
                content,
                client_message_id: None,
            },
    } = &record.event
    {
        return has(content);
    }
    if let SessionEvent::QueueChanged { queue } = &record.event {
        return queue.entries.iter().any(|entry| has(&entry.content));
    }
    false
}

/// Read `told`'s log until a prompt with `words` arrives in it.
async fn told(stream: &mut SessionStream, seen: &mut Vec<EventRecord>, words: &[&str]) {
    if seen.iter().any(|record| says(record, words)) {
        return;
    }
    read_until(stream, seen, |record| says(record, words)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_is_told_when_its_childs_turn_finishes_waits_and_ends() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut first = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut first).await;
        let agent: NodeClient = as_agent(&socket, &auth).await;
        let child = agent
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        let _ready = following(&owner, child, EventSeq::ZERO).await;
        let mut stream = owner.follow(first.id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();

        // A turn that finishes: its end and its reply's end, in the told
        // session (which was idle: the notice starts its turn).
        assert_eq!(
            agent
                .prompt_told(child, text("plain"), Delivery::Queue)
                .await
                .unwrap(),
            Delivered::Started
        );
        let child_id = child.to_string();
        let finished = [
            child_id.as_str(),
            "finished its turn (end_turn).",
            "echo: plain",
        ];
        told(&mut stream, &mut seen, &finished).await;

        // A turn that asks for a permission: told once it waits.
        let _started = agent
            .prompt_told(child, text("tool"), Delivery::Queue)
            .await
            .unwrap();
        let waits = [
            child_id.as_str(),
            "is waiting for an answer to a permission for",
            "\nRun fake tool\n</vessel-session-output-",
        ];
        told(&mut stream, &mut seen, &waits).await;

        // A prompt queued behind that waiting turn never starts one. The
        // child is stopped: the waiting turn is told as failed, and the
        // queued prompt as the session's end.
        let queued = agent
            .prompt_told(child, text("plain"), Delivery::Queue)
            .await
            .unwrap();
        assert!(matches!(queued, Delivered::Queued { .. }), "{queued:?}");
        owner.kill(child).await.unwrap();
        let failed = [child_id.as_str(), "could not finish its turn"];
        told(&mut stream, &mut seen, &failed).await;
        told(&mut stream, &mut seen, &[child_id.as_str(), "ended ("]).await;

        // Nobody but a session's agent asks for it.
        let other: SessionId = structured(&owner, &fixture).await.id;
        assert_eq!(
            code(
                owner
                    .prompt_told(other, text("plain"), Delivery::Queue)
                    .await
            ),
            ErrorCode::InvalidRequest
        );
        node.stop().await.unwrap();
    })
    .await;
}
