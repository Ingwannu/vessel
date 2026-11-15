//! Default completion notices cross the actual MCP decoder and node queue.
//! Fake agents only: no model transport or real user home is touched.

use serde_json::json;
use vessel_client::SessionStream;
use vessel_proto::{AgentEvent, ContentBlock, SessionEvent, SessionId};

use support::{Fixture, as_agent, owner, within};

#[cfg(test)]
#[path = "support/mcp.rs"]
mod support;

async fn completion(stream: &mut SessionStream, child: SessionId, reply: &str) {
    let child = child.to_string();
    while let Some(record) = stream.next().await {
        if let SessionEvent::Agent {
            event: AgentEvent::TurnStarted { content },
        } = record.unwrap().event
            && content.iter().any(|block| {
                matches!(block, ContentBlock::Text { text }
                    if text.contains(&child)
                        && text.contains("finished its turn (end_turn)")
                        && text.contains(reply))
            })
        {
            return;
        }
    }
    panic!("parent ended before its child's completion notice");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_create_and_send_notify_the_calling_agent_without_wait_tools() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (mut mcp, server, own) = as_agent(&owner).await;
        let from = owner.session(own).await.unwrap().next_seq;
        let mut parent = owner.follow(own, from).await.unwrap();
        let (failed, created) = mcp
            .call(
                "create_session",
                json!({"harness":"claude","prompt":"plain"}),
            )
            .await;
        assert!(!failed, "{created}");
        let child: SessionId = serde_json::from_value(created["session"]["id"].clone()).unwrap();
        completion(&mut parent, child, "echo: plain").await;
        let (failed, sent) = mcp
            .call("send_input", json!({"session":child,"text":"plain again"}))
            .await;
        assert!(!failed, "{sent}");
        completion(&mut parent, child, "echo: plain again").await;
        drop(mcp);
        server.await.unwrap().unwrap();
        owner.kill(child).await.unwrap();
        owner.kill(own).await.unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}
