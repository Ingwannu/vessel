//! The conversation's agent tools (protocol 10) end to end, through a node
//! in this process with `fake-acp` as Claude's adapter: a prompt queued
//! and steered while a turn runs, the queue read and changed, a session
//! archived and started again, a question answered; and what the node
//! refuses an agent.
//!
//! Tests wait on the session's records (`wait_turn` reads them), never on
//! time; the timeout only turns a hang into a failure.

use serde_json::{Value, json};
use vessel_mcp::VesselTools;
use vessel_proto::{ContentBlock, SessionInput};

use support::{Fixture, agent_env, as_agent, owner, serve, within};

#[cfg(test)]
#[path = "support/mcp.rs"]
mod support;

/// A refusal's text starts with the node's error code, as JSON.
fn refused(failed: bool, refusal: &Value, code: &str) {
    assert!(failed, "{refusal}");
    let text = refusal.as_str().unwrap_or_default();
    assert!(text.starts_with(&format!("\"{code}\"")), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_queues_and_steers_prompts() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (mut mcp, server, own_session) = as_agent(&owner).await;

        let (failed, created) = mcp
            .call(
                "create_session",
                json!({ "harness": "claude", "cwd": std::env::temp_dir() }),
            )
            .await;
        assert!(!failed, "{created}");
        let session = created["session"]["id"].clone();

        // A turn that runs until it is steered to finish; a prompt queued
        // behind it.
        let (failed, sent) = mcp
            .call("send_input", json!({ "session": session, "text": "wait" }))
            .await;
        assert!(!failed, "{sent}");
        let from = sent["from"].clone();
        let (failed, queued) = mcp
            .call(
                "send_input",
                json!({ "session": session, "text": "plian", "delivery": "queue" }),
            )
            .await;
        assert!(!failed, "{queued}");
        assert_eq!(queued["delivered"]["delivered"], "queued", "{queued}");
        let entry = queued["delivered"]["entry"].clone();

        let (failed, conversation) = mcp
            .call("read_conversation", json!({ "session": session }))
            .await;
        assert!(!failed, "{conversation}");
        assert_eq!(conversation["queue"]["entries"][0]["id"], entry);
        assert_eq!(conversation["abilities"]["steer"], true, "{conversation}");

        // Its text corrected, then the queue held past the turn's end.
        let (failed, edited) = mcp
            .call(
                "change_queue",
                json!({ "session": session, "change": "edit", "entry": entry, "text": "plain" }),
            )
            .await;
        assert!(!failed, "{edited}");
        assert_eq!(edited["queue"]["entries"][0]["revision"], 1, "{edited}");
        let (failed, held) = mcp
            .call(
                "change_queue",
                json!({ "session": session, "change": "hold" }),
            )
            .await;
        assert!(!failed, "{held}");
        assert_eq!(held["queue"]["held"], "asked", "{held}");

        let (failed, steered) = mcp
            .call(
                "send_input",
                json!({ "session": session, "text": "finish", "delivery": "steer" }),
            )
            .await;
        assert!(!failed, "{steered}");
        assert_eq!(steered["delivered"]["delivered"], "steered", "{steered}");
        let (failed, turn) = mcp
            .call(
                "wait_turn",
                json!({ "session": session, "from": from, "timeout_secs": 30 }),
            )
            .await;
        assert!(!failed, "{turn}");
        assert_eq!(turn["outcome"], "ended", "{turn}");

        // Let go, the queue sends the corrected prompt.
        let (failed, resumed) = mcp
            .call(
                "change_queue",
                json!({ "session": session, "change": "resume" }),
            )
            .await;
        assert!(!failed, "{resumed}");
        assert_eq!(resumed["delivered"]["delivered"], "started", "{resumed}");
        let (failed, turn) = mcp
            .call(
                "wait_turn",
                json!({ "session": session, "from": turn["next"], "timeout_secs": 30 }),
            )
            .await;
        assert!(!failed, "{turn}");
        assert_eq!(turn["reply"], "echo: plain", "{turn}");
        let (failed, gone) = mcp
            .call(
                "change_queue",
                json!({ "session": session, "change": "remove", "entry": entry }),
            )
            .await;
        refused(failed, &gone, "not_found");

        let (failed, _killed) = mcp
            .call("kill_session", json!({ "session": session }))
            .await;
        assert!(!failed);
        drop(mcp);
        server.await.unwrap().unwrap();
        owner.kill(own_session).await.unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_archives_a_session_and_starts_it_again() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (mut mcp, server, own_session) = as_agent(&owner).await;
        let (failed, created) = mcp
            .call(
                "create_session",
                json!({ "harness": "claude", "cwd": std::env::temp_dir(), "prompt": "plain" }),
            )
            .await;
        assert!(!failed, "{created}");
        let session = created["session"]["id"].clone();
        let (failed, turn) = mcp
            .call(
                "wait_turn",
                json!({ "session": session, "from": created["from"], "timeout_secs": 30 }),
            )
            .await;
        assert!(!failed, "{turn}");

        // Put away (stopped first), then started again in the same log.
        let (failed, archived) = mcp
            .call("archive_session", json!({ "session": session }))
            .await;
        assert!(!failed, "{archived}");
        assert_ne!(
            archived["session"]["status"]["state"], "running",
            "{archived}"
        );
        assert!(
            archived["session"]["marks"]["archived_at"].is_string(),
            "{archived}"
        );
        let (failed, again) = mcp
            .call("resume_session", json!({ "session": session }))
            .await;
        assert!(!failed, "{again}");
        assert_eq!(again["session"]["status"]["state"], "running", "{again}");
        assert!(
            again["session"]["marks"]["archived_at"].is_null(),
            "{again}"
        );

        // Not its own session: the node says why.
        let (failed, refusal) = mcp
            .call("archive_session", json!({ "session": own_session }))
            .await;
        refused(failed, &refusal, "forbidden");
        // Words the tools do not know: errors of the call.
        for (tool, arguments) in [
            (
                "send_input",
                json!({ "session": session, "text": "x", "delivery": "sideways" }),
            ),
            (
                "change_queue",
                json!({ "session": session, "change": "shuffle" }),
            ),
            (
                "answer_question",
                json!({ "session": session, "question": "q", "values": {} }),
            ),
        ] {
            let response = mcp
                .request(
                    "tools/call",
                    json!({ "name": tool, "arguments": arguments }),
                )
                .await;
            assert!(response["error"].is_object(), "{tool}: {response}");
        }

        let (failed, _killed) = mcp
            .call("kill_session", json!({ "session": session }))
            .await;
        assert!(!failed);
        drop(mcp);
        server.await.unwrap().unwrap();
        owner.kill(own_session).await.unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}

/// A question the owner's session asks: an agent sees it and is refused
/// the answer (the session is not its own); the same tool through the
/// owner's connection reads each value by its field and answers it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_question_is_answered_with_the_tool_by_whoever_may() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (asking, env) = agent_env(&owner).await;
        let from = owner.session(asking).await.unwrap().next_seq;
        let ask = SessionInput::Prompt {
            content: vec![ContentBlock::Text {
                text: "ask".to_owned(),
            }],
        };
        owner.send(asking, ask).await.unwrap();

        let (mut agent, agent_server) =
            serve(VesselTools::new(vessel_mcp::connect(&env).await.unwrap()));
        let _names = agent.initialize().await;
        let (failed, asked) = agent
            .call(
                "wait_turn",
                json!({ "session": asking, "from": from, "timeout_secs": 30 }),
            )
            .await;
        assert!(!failed, "{asked}");
        assert_eq!(asked["outcome"], "question_asked", "{asked}");
        let question = asked["question"]["id"].clone();
        let (failed, refusal) = agent
            .call(
                "answer_question",
                json!({ "session": asking, "question": question, "values": { "way": "left" } }),
            )
            .await;
        refused(failed, &refusal, "forbidden");

        let (mut person, person_server) = serve(VesselTools::new(support::owner(&fixture).await));
        let _names = person.initialize().await;
        let (failed, unreadable) = person
            .call(
                "answer_question",
                json!({ "session": asking, "question": question, "values": { "sure": "maybe" } }),
            )
            .await;
        assert!(failed, "{unreadable}");
        assert!(
            unreadable
                .as_str()
                .unwrap()
                .contains("`sure` takes yes or no"),
            "{unreadable}"
        );
        let (failed, wrong) = person
            .call(
                "answer_question",
                json!({ "session": asking, "question": question, "values": { "way": "up" } }),
            )
            .await;
        refused(failed, &wrong, "invalid_request");
        let (failed, answered) = person
            .call(
                "answer_question",
                json!({
                    "session": asking,
                    "question": question,
                    "values": { "way": "left", "sure": true }
                }),
            )
            .await;
        assert!(!failed, "{answered}");
        let (failed, again) = person
            .call(
                "answer_question",
                json!({ "session": asking, "question": question, "decline": true }),
            )
            .await;
        refused(failed, &again, "conflict");

        let (failed, turn) = person
            .call(
                "wait_turn",
                json!({ "session": asking, "from": asked["next"], "timeout_secs": 30 }),
            )
            .await;
        assert!(!failed, "{turn}");
        assert_eq!(
            turn["reply"], r#"answered: accept {"sure":true,"way":"left"}"#,
            "{turn}"
        );

        drop(agent);
        drop(person);
        agent_server.await.unwrap().unwrap();
        person_server.await.unwrap().unwrap();
        owner.kill(asking).await.unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}
