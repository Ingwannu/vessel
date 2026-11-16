//! Exercise selector decoding and pending admission through the real MCP pipe.
use serde_json::json;

use support::{Fixture, as_agent, owner, within};

#[cfg(test)]
#[path = "support/mcp.rs"]
mod support;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovery_preserves_explicit_selectors_and_validates_before_lookup() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (mut mcp, server, own) = as_agent(&owner).await;
        let before = owner.sessions().await.unwrap().len();
        for args in [json!({"scope":"all"}), json!({"labels":{}})] {
            let (failed, reason) = mcp.call("list_sessions", args).await;
            assert!(failed, "{reason}");
            assert!(reason.as_str().unwrap().contains("protocol version is not assigned"));
        }
        let malformed = mcp
            .request(
                "tools/call",
                json!({"name":"list_sessions","arguments":{"machine":"absent-machine","scope":null}}),
            )
            .await;
        assert_eq!(malformed["error"]["code"], -32602);
        let (failed, sessions) = mcp.call("list_sessions", json!({})).await;
        assert!(!failed, "{sessions}");
        assert_eq!(sessions.as_array().unwrap().len(), before);
        drop(mcp);
        server.await.unwrap().unwrap();
        owner.kill(own).await.unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}
