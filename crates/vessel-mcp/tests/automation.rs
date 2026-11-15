//! The schedule and saved-prompt tools (protocol 11, ADR 0033) end to end,
//! through a node in this process with `fake-acp` as Claude's adapter: an
//! agent keeps a heartbeat on its own session and a schedule that starts
//! sessions, lists, pauses, resumes and deletes them, reads their runs and
//! the saved prompts, and is refused what it may not do.

use serde_json::{Value, json};
use vessel_proto::automation::CreatePrompt;

use support::{Fixture, as_agent, owner, within};

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
async fn an_agent_keeps_schedules_of_its_own_and_reads_saved_prompts() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (mut mcp, server, own_session) = as_agent(&owner).await;

        // A heartbeat: its own session, through its queue.
        let (failed, heartbeat) = mcp
            .call(
                "create_heartbeat",
                json!({ "every_secs": 3600, "prompt": "look again", "name": "pulse" }),
            )
            .await;
        assert!(!failed, "{heartbeat}");
        assert_eq!(
            heartbeat["definition"]["target"]["session"],
            json!(own_session),
            "{heartbeat}"
        );
        assert_eq!(heartbeat["defined_by"]["session"], json!(own_session));

        // A schedule that starts a session each run, on a cron in a zone.
        let (failed, nightly) = mcp
            .call(
                "create_schedule",
                json!({
                    "cron": "0 9 * * 1-5", "tz": "Asia/Seoul", "prompt": "plain",
                    "harness": "claude", "cwd": std::env::temp_dir()
                }),
            )
            .await;
        assert!(!failed, "{nightly}");
        assert_eq!(nightly["state"]["state"], "active", "{nightly}");
        replace_schedule_and_refuse_stale(&mut mcp, &nightly).await;

        let (failed, listed) = mcp.call("list_schedules", json!({})).await;
        assert!(!failed);
        assert_eq!(listed["schedules"].as_array().map(Vec::len), Some(2));
        let id = heartbeat["id"].clone();
        let (failed, paused) = mcp.call("pause_schedule", json!({ "schedule": id })).await;
        assert!(!failed, "{paused}");
        assert_eq!(paused["state"]["state"], "paused");
        let (failed, resumed) = mcp.call("resume_schedule", json!({ "schedule": id })).await;
        assert!(!failed, "{resumed}");
        let (failed, runs) = mcp.call("schedule_runs", json!({ "schedule": id })).await;
        assert!(!failed, "{runs}");
        assert_eq!(runs["runs"], json!([]));
        let (failed, _deleted) = mcp.call("delete_schedule", json!({ "schedule": id })).await;
        assert!(!failed);

        // A person's schedule is not the agent's to change.
        let persons = owner
            .create_schedule(
                serde_json::from_value(json!({
                    "cadence": { "kind": "every", "seconds": 3600 },
                    "target": { "kind": "session", "session": own_session },
                    "prompt": "from a person", "catch_up": "once"
                }))
                .unwrap(),
                false,
            )
            .await
            .unwrap();
        let (failed, refusal) = mcp
            .call("pause_schedule", json!({ "schedule": persons.id }))
            .await;
        refused(failed, &refusal, "forbidden");

        // Saved prompts: read, never written (no tool writes one).
        let saved = owner
            .create_prompt(CreatePrompt {
                name: "review".parse().unwrap(),
                text: "Review the open pull requests.".parse().unwrap(),
                project: None,
                session: None,
            })
            .await
            .unwrap();
        let (failed, prompts) = mcp.call("list_saved_prompts", json!({})).await;
        assert!(!failed, "{prompts}");
        assert_eq!(prompts["prompts"][0]["id"], json!(saved.id));

        drop(mcp);
        server.await.unwrap().unwrap();
        owner.kill(own_session).await.unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}

#[cfg(test)]
async fn replace_schedule_and_refuse_stale(mcp: &mut support::McpClient, nightly: &Value) {
    let (failed, inspected) = mcp
        .call("get_schedule", json!({ "schedule": nightly["id"] }))
        .await;
    assert!(!failed, "{inspected}");
    assert_eq!(inspected["revision"], nightly["revision"]);
    let (failed, updated) = mcp
        .call(
            "update_schedule",
            json!({
                "schedule": nightly["id"], "revision": nightly["revision"],
                "cron": "0 9 * * 1-5", "tz": "Asia/Seoul", "prompt": "plain again",
                "harness": "claude", "cwd": std::env::temp_dir()
            }),
        )
        .await;
    assert!(!failed, "{updated}");
    assert_eq!(updated["definition"]["prompt"], "plain again");
    let (failed, stale) = mcp
        .call(
            "update_schedule",
            json!({
                "schedule": nightly["id"], "revision": nightly["revision"],
                "cron": "0 9 * * 1-5", "tz": "Asia/Seoul", "prompt": "too late",
                "harness": "claude", "cwd": std::env::temp_dir()
            }),
        )
        .await;
    refused(failed, &stale, "conflict");
}
