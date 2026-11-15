//! Agent-facing metadata refuses before node work; legacy discovery stays usable.

use serde_json::json;
use support::{Fixture, as_agent, owner, within};
use vessel_client::CallError;
use vessel_proto::{InvalidLabel, Method};

#[cfg(test)]
#[path = "support/mcp.rs"]
#[expect(
    dead_code,
    reason = "only the shared MCP fixture and agent setup are needed"
)]
mod support;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_pending_metadata_never_creates_a_session_or_schedule() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (mut mcp, server, parent) = as_agent(&owner).await;
        let before = owner.sessions().await.unwrap().len();
        let (failed, listed) = mcp.call("list_sessions", json!({})).await;
        assert!(!failed, "{listed}");
        assert_eq!(listed.as_array().unwrap().len(), before);
        for (tool, args, method) in [
            ("list_sessions", json!({"scope":"all"}), Method::ListSessions),
            ("list_sessions", json!({"labels":{"env":"prod"}}), Method::ListSessions),
            ("create_session", json!({"harness":"claude","listing":"shown"}), Method::CreateSession),
            ("create_schedule", json!({"harness":"claude","every_secs":vessel_proto::automation::EverySeconds::MIN,"prompt":"fixture","listing":"background"}), Method::CreateSchedule),
        ] {
            let (failed, refusal) = mcp.call(tool, args).await;
            assert!(failed, "{refusal}");
            assert_eq!(refusal.as_str().unwrap(), CallError::UnassignedProtocol { method }.to_string());
        }
        assert_eq!(owner.sessions().await.unwrap().len(), before);
        assert!(owner.schedules().await.unwrap().is_empty());
        drop(mcp);
        server.await.unwrap().unwrap();
        owner.kill(parent).await.unwrap();
        fixture.node.stop().await.unwrap();
    }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_metadata_precedes_target_lookup_and_existing_target_overrides_are_errors() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (mut mcp, server, parent) = as_agent(&owner).await;
        for tool in ["list_sessions", "create_session"] {
            let response = mcp.request("tools/call", json!({"name":tool,"arguments":{
                "node":"missing-target", "harness":"claude", "labels":{"":"invalid"}
            }})).await;
            let message = response["error"]["message"].as_str().unwrap();
            assert!(message.contains(&InvalidLabel::EmptyKey.to_string()), "{message}");
            assert!(!message.contains("missing-target"), "target lookup ran first: {message}");
        }
        for (tool, args) in [
            ("create_schedule", json!({"session":parent,"every_secs":vessel_proto::automation::EverySeconds::MIN,"prompt":"fixture","labels":{}})),
            ("create_heartbeat", json!({"every_secs":vessel_proto::automation::EverySeconds::MIN,"prompt":"fixture","listing":"shown"})),
        ] {
            let response = mcp.request("tools/call", json!({"name":tool,"arguments":args})).await;
            assert!(response["error"].is_object(), "{response}");
        }
        assert!(owner.schedules().await.unwrap().is_empty());
        drop(mcp);
        server.await.unwrap().unwrap();
        owner.kill(parent).await.unwrap();
        fixture.node.stop().await.unwrap();
    }).await;
}
