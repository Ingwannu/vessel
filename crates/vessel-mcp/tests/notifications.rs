//! Default completion notices cross the actual MCP decoder and node queue.
//! Fake agents only: no model transport or real user home is touched.

use std::{future::Future, time::Duration};

use serde_json::json;
use vessel_client::SessionStream;
use vessel_proto::{AgentEvent, ContentBlock, SessionEvent, SessionId};

use support::{Fixture, as_agent, owner, within};

#[cfg(test)]
#[path = "support/mcp.rs"]
mod support;

/// One stage may stall without letting this real MCP exchange hang the suite.
const NOTICE_STEP_BUDGET: Duration = Duration::from_secs(15);

#[cfg(test)]
async fn step<T>(name: &str, work: impl Future<Output = T>) -> T {
    tokio::time::timeout(NOTICE_STEP_BUDGET, work)
        .await
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[cfg(test)]
async fn completion(stream: &mut SessionStream, child: SessionId, reply: &str) {
    let child = child.to_string();
    let contains = |content: &[ContentBlock]| {
        content.iter().any(|block| {
            matches!(block, ContentBlock::Text { text }
                if text.contains(&child)
                    && text.contains("finished its turn (end_turn)")
                    && text.contains(reply))
        })
    };
    while let Some(record) = stream.next().await {
        // An occupied parent receives a queued notice; it need not start a
        // second turn before the notice is observable in its own queue.
        let received = match record.unwrap().event {
            SessionEvent::Agent {
                event: AgentEvent::TurnStarted { content },
            } => contains(&content),
            SessionEvent::QueueChanged { queue } => {
                queue.entries.iter().any(|entry| contains(&entry.content))
            }
            SessionEvent::Status { .. }
            | SessionEvent::Terminal(_)
            | SessionEvent::Agent { .. }
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. } => false,
        };
        if received {
            return;
        }
    }
    panic!("parent ended before its child's completion notice");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_create_and_send_notify_the_calling_agent_without_wait_tools() {
    within(async {
        let fixture = Fixture::start().await;
        // A session may not use the node's private data directory as cwd.
        let placement = tempfile::tempdir().unwrap();
        let owner = owner(&fixture).await;
        let (mut mcp, server, own) = as_agent(&owner).await;
        let from = owner.session(own).await.unwrap().next_seq;
        let mut parent = owner.follow(own, from).await.unwrap();
        let (failed, created) = step(
            "create with default notify",
            mcp.call(
                "create_session",
                json!({"harness":"claude","cwd":placement.path(),"prompt":"plain"}),
            ),
        )
        .await;
        assert!(!failed, "{created}");
        let child: SessionId = serde_json::from_value(created["session"]["id"].clone()).unwrap();
        step(
            "first completion notice",
            completion(&mut parent, child, "echo: plain"),
        )
        .await;
        // The fake's arbitrary-text path asks permission; cwd is a distinct
        // deterministic completed reply, without weakening the second notice.
        let (failed, sent) = step(
            "send with default delivery and notify",
            mcp.call("send_input", json!({"session":child,"text":"cwd"})),
        )
        .await;
        assert!(!failed, "{sent}");
        let expected = format!("cwd {}", placement.path().display());
        step(
            "second completion notice",
            completion(&mut parent, child, &expected),
        )
        .await;
        drop(mcp);
        server.await.unwrap().unwrap();
        owner.kill(child).await.unwrap();
        owner.kill(own).await.unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}
