//! A hidden tool must be refused before its node mutation, even by direct name.

use serde_json::json;
use vessel_mcp::VesselTools;
use vessel_proto::mcp::{StoredTool, ToolAccess, ToolId, ToolPolicy};

use support::{Fixture, agent_env, owner, serve, within};

#[cfg(test)]
#[path = "support/mcp.rs"]
#[expect(dead_code, reason = "the shared fixture, not every helper of it")]
mod support;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_disabled_creation_cannot_bypass_the_catalogue() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (own, env) = agent_env(&owner).await;
        let policy = ToolPolicy::new(ToolAccess::Enabled, vec![StoredTool::Known(ToolId::CreateSession), StoredTool::Known(ToolId::Browser(vessel_proto::browser::BrowserTool::Click))]).unwrap();
        let tools = VesselTools::new(vessel_mcp::connect(&env).await.unwrap()).with_policy(policy);
        let (mut mcp, task) = serve(tools);
        let names = mcp.initialize().await;
        assert!(!names.iter().any(|name| name == "create_session"));
        assert!(!names.iter().any(|name| name.starts_with("browser_")));
        let (failed, reason) = mcp.call("browser_click", json!({})).await;
        assert!(failed, "{reason}");
        assert!(reason.as_str().unwrap().contains("disabled"));
        let (failed, reason) = mcp.call("browser_type", json!({})).await;
        assert!(failed, "{reason}");
        assert!(reason.as_str().unwrap().contains("qualified"));
        let before = owner.sessions().await.unwrap().len();
        let (failed, reason) = mcp.call("create_session", json!({"harness":"claude"})).await;
        assert!(failed, "{reason}");
        assert!(reason.as_str().unwrap().contains("disabled"));
        assert_eq!(owner.sessions().await.unwrap().len(), before);
        let (failed, sessions) = mcp.call("list_sessions", json!({"machine":fixture.node.info().id.to_string()})).await;
        assert!(!failed, "{sessions}");
        assert_eq!(sessions.as_array().unwrap().len(), before);
        let malformed = mcp.request("tools/call", json!({"name":"create_heartbeat","arguments":{"machine":"elsewhere","every_secs":3600,"prompt":"no"}})).await;
        assert_eq!(malformed["error"]["code"], -32602);
        assert!(owner.schedules().await.unwrap().is_empty());
        drop(mcp);
        task.await.unwrap().unwrap();
        owner.kill(own).await.unwrap();
        fixture.node.stop().await.unwrap();
    }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_creation_options_never_leave_a_session_behind() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (own, env) = agent_env(&owner).await;
        let tools =
            VesselTools::new(vessel_mcp::connect(&env).await.unwrap()).with_own_session(own);
        let (mut mcp, task) = serve(tools);
        let names = mcp.initialize().await;
        assert!(!names.iter().any(|name| name == "wait_turn"));
        let before = owner.sessions().await.unwrap().len();
        for arguments in [
            json!({"harness":"claude","prompt":42}),
            json!({"harness":"claude","prompt":"x","notify":"yes"}),
            json!({"harness":"claude","prompt":"x","terminal":true,"notify":true}),
            json!({"harness":"claude","notify":true}),
        ] {
            let response = mcp
                .request(
                    "tools/call",
                    json!({"name":"create_session","arguments":arguments}),
                )
                .await;
            assert_eq!(response["error"]["code"], -32602, "{response}");
            assert_eq!(owner.sessions().await.unwrap().len(), before);
        }
        drop(mcp);
        task.await.unwrap().unwrap();
        owner.kill(own).await.unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}
