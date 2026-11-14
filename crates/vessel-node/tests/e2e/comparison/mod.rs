//! Existing-call fanout against real nodes, `SQLite` and fake structured agents.
mod access;
mod delivery;
mod faults;
mod observation;
mod preflight;
mod source_changes;

use crate::support::{Fixture, step};
use bytes::Bytes;
use std::sync::Arc;
use vessel_client::{
    NodeClient,
    comparison::{
        ComparisonInput, ComparisonJob, ComparisonPlan, Selection, Snapshot, SourceAttachments,
        TargetStatus,
    },
};
use vessel_proto::workspace::WorktreeBase;
use vessel_proto::{AttachmentRef, EventSeq, HarnessKind, ProjectId, SessionId};

fn plan(project: ProjectId, models: &[&str]) -> ComparisonPlan {
    ComparisonPlan::new(
        project,
        WorktreeBase::Ref {
            name: "main".parse().unwrap(),
        },
        models
            .iter()
            .map(|model| {
                Selection::new(
                    HarnessKind::ClaudeCode.into(),
                    None,
                    Some(vessel_proto::ModelId::new(*model)),
                    Vec::new(),
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap()
}
async fn prepared(client: &NodeClient, project: ProjectId, models: &[&str]) -> ComparisonJob {
    let job = client.comparison(client.node().id, plan(project, models));
    let snapshot = step("comparison prepare", job.prepare()).await.unwrap();
    assert!(
        snapshot
            .targets
            .iter()
            .all(|entry| matches!(entry.status, TargetStatus::Ready)),
        "{snapshot:?}"
    );
    job
}
async fn attached(client: &NodeClient, session: SessionId, bytes: Bytes) -> AttachmentRef {
    client
        .attach(
            session,
            "common.txt".parse().unwrap(),
            "text/plain".parse().unwrap(),
            bytes,
        )
        .await
        .unwrap()
}
fn input(text: &str, source: Option<(SessionId, AttachmentRef)>) -> ComparisonInput {
    ComparisonInput::new(
        Some(text.parse().unwrap()),
        source.map(|(session, file)| SourceAttachments::new(session, vec![file]).unwrap()),
    )
    .unwrap()
}
async fn turns(client: &NodeClient, session: SessionId) -> Vec<Vec<vessel_proto::ContentBlock>> {
    client
        .read(session, EventSeq::ZERO, 1000)
        .await
        .unwrap()
        .records
        .into_iter()
        .filter_map(|record| {
            if let vessel_proto::SessionEvent::Agent {
                event: vessel_proto::AgentEvent::TurnStarted { content },
            } = record.event
            {
                Some(content)
            } else {
                None
            }
        })
        .collect()
}
async fn changed(job: &ComparisonJob, predicate: impl Fn(&Snapshot) -> bool) -> Arc<Snapshot> {
    let mut updates = job.updates();
    step("comparison checkpoint", async {
        loop {
            let snapshot = updates.borrow_and_update().clone();
            if predicate(&snapshot) {
                return snapshot;
            }
            updates.changed().await.unwrap();
        }
    })
    .await
}
/// The node, with a silence limit beyond any clock jump here: the deadline
/// tests (`faults/deadlines.rs`) pause the node's clock and advance it 46 s
/// and 121 s at once, past the 30 s after which the node drops a client it
/// has not heard from (`/rpc` and `/secure` alike), and no pong can come
/// back across a jump. Keepalive is not what these tests are about.
async fn start_node(fixture: &Fixture) -> vessel_node::Node {
    fixture
        .start_with(|config| {
            config.harness_lookup = crate::workspace::lookup(fixture);
            config.keepalive = vessel_relay::Keepalive {
                ping: std::time::Duration::from_secs(10),
                silence: std::time::Duration::from_secs(3_600),
            };
        })
        .await
}
