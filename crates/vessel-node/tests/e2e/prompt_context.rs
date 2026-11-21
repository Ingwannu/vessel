//! A prompt's context and review blocks (ADR 0124): the agent reads the
//! text the node renders for each, the log keeps the typed blocks for a
//! timeline's pills, and a block past its bounds is refused before anything
//! is sent.

use vessel_proto::prompt_context::{MAX_COMMENTS, agent_text};
use vessel_proto::{AgentEvent, ContentBlock, Delivery, ErrorCode};

use crate::agents::code;
use crate::attachments::{answer, running, text};
use crate::support::{Fixture, agent_of, step, within};

fn context() -> ContentBlock {
    serde_json::from_value(serde_json::json!({
        "type": "context",
        "kind": "pull_comment",
        "title": "mkim",
        "text": "GitHub pull request comment\nShould this cap at 30 seconds?",
        "url": "https://github.com/acme-labs/fleet-agent/pull/128#discussion_RC_1",
    }))
    .unwrap()
}

fn review(comments: usize) -> ContentBlock {
    let comment = serde_json::json!({
        "path": "src/upload/retry.rs",
        "side": "new",
        "line": 48,
        "body": "Cap this at 30 seconds.",
        "hunk_header": "@@ -40,6 +40,9 @@ fn backoff",
        "target": { "new": 48, "kind": "add", "content": "sleep(delay);" },
        "lines": [{ "new": 48, "kind": "add", "content": "sleep(delay);" }],
    });
    serde_json::from_value(serde_json::json!({
        "type": "review",
        "root": "/work/fleet-agent",
        "mode": "uncommitted",
        "comments": vec![comment; comments],
    }))
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_agent_reads_a_context_and_a_review_as_text_and_the_log_keeps_them_typed() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, mut stream, mut seen) = running(&client, &fixture, None).await;
        let start = seen.len();
        let said = answer(
            &client,
            (&mut stream, &mut seen),
            id,
            vec![text("describe"), context(), review(1)],
        )
        .await;
        // The agent was given three text blocks: the words, then each
        // block's text as the node renders it.
        let context_text = agent_text(&context()).unwrap();
        let review_text = agent_text(&review(1)).unwrap();
        assert_eq!(
            said,
            format!(
                "text 8; text {}; text {}",
                context_text.len(),
                review_text.len()
            )
        );
        assert!(review_text.starts_with("Vessel review attachment (uncommitted)\n"));
        // The turn's prompt stays as it was sent.
        let started = seen[start..]
            .iter()
            .find_map(|record| match agent_of(record) {
                Some(AgentEvent::TurnStarted { content }) => Some(content.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(started, vec![text("describe"), context(), review(1)]);
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_review_past_its_bounds_is_refused_before_it_is_sent() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, _stream, _seen) = running(&client, &fixture, None).await;
        let refused = step(
            "a review of too many comments",
            client.prompt(
                id,
                vec![text("plain"), review(MAX_COMMENTS.saturating_add(1))],
                Delivery::Start,
            ),
        )
        .await;
        assert_eq!(code(refused), ErrorCode::InvalidRequest);
    })
    .await;
}
