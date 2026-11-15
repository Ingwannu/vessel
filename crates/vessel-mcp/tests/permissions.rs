//! Aggregate current conversation state rather than treating old log events as pending.

use serde_json::json;
use support::{Fixture, as_agent, owner, within};

#[cfg(test)]
#[path = "support/mcp.rs"]
#[expect(dead_code, reason = "the shared fixture, not every helper of it")]
mod support;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn answered_permissions_disappear_without_erasing_their_log() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (mut mcp, task, own) = as_agent(&owner).await;
        let (failed, created) = mcp.call("create_session", json!({"harness":"claude","cwd":std::env::temp_dir(),"prompt":"permit plan"})).await;
        assert!(!failed, "{created}");
        let session = created["session"]["id"].clone();
        let (failed, waiting) = mcp.call("wait_turn", json!({"session":session,"from":created["from"]})).await;
        assert!(!failed, "{waiting}");
        assert_eq!(waiting["outcome"], "permission_requested");
        let (failed, pending) = mcp.call("list_pending_permissions", json!({})).await;
        assert!(!failed, "{pending}");
        assert_eq!(pending["permissions"].as_array().unwrap().len(), 1);
        assert_eq!(pending["permissions"][0]["session"], session);
        assert_eq!(pending["permissions"][0]["request"]["id"], waiting["request"]["id"]);
        let (failed, answered) = mcp.call("answer_permission", json!({"session":session,"request":waiting["request"]["id"],"option":waiting["request"]["options"][0]["id"]})).await;
        assert!(!failed, "{answered}");
        let (failed, ended) = mcp.call("wait_turn", json!({"session":session,"from":waiting["next"]})).await;
        assert!(!failed, "{ended}");
        assert_eq!(ended["outcome"], "ended");
        let (failed, pending) = mcp.call("list_pending_permissions", json!({})).await;
        assert!(!failed, "{pending}");
        assert_eq!(pending["permissions"], json!([]));
        let (failed, log) = mcp.call("read_events", json!({"session":session})).await;
        assert!(!failed, "{log}");
        assert!(log.to_string().contains("permission_requested"));
        let (failed, killed) = mcp.call("kill_session", json!({"session":session})).await;
        assert!(!failed, "{killed}");
        drop(mcp);
        task.await.unwrap().unwrap();
        owner.kill(own).await.unwrap();
        fixture.node.stop().await.unwrap();
    }).await;
}
