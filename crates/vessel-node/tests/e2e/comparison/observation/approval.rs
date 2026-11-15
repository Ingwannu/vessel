//! Pending approval survives first dispatch; continued create is consumed before IO.
use super::*;
use vessel_client::comparison::{FailureKind, JobError, Phase};
use vessel_proto::Method;

enum Outcome {
    CompleteAfterDroppedReply,
    CancelCreate,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn approval_continuations_keep_common_input_and_uncertain_create_cannot_repeat() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (folder, project) = crate::tidying::project(&owner).await;
        std::fs::write(folder.path().join("vessel.toml"), "[worktree]\nsetup = \"echo ready\"\n").unwrap();
        commit(folder.path(), "vessel.toml");
        for outcome in [Outcome::CompleteAfterDroppedReply, Outcome::CancelCreate] {
            approve(&owner, project.id, true).await;
            let bridge = Bridge::open(&fixture, &node).await;
            let job = bridge.client.comparison(node.info().id, plan(project.id, &["fake-small", "fake-large"]));
            let arrival = bridge.hold(Method::CreateWorktree).await;
            let preparing = job.prepare();
            tokio::pin!(preparing);
            tokio::select! {
                reply = &mut preparing => panic!("held worktree completed {reply:?}"),
                held = arrival => held.unwrap(),
            }
            // This response confirms a real first worktree; withdrawing approval
            // affects only the next selection's ordinary creation guard.
            approve(&owner, project.id, false).await;
            bridge.release().await;
            let prepared = preparing.await.unwrap();
            assert!(matches!(prepared.targets[0].status, TargetStatus::Ready));
            assert!(matches!(prepared.targets[1].status, TargetStatus::NeedsApproval(_)));
            let source = prepared.targets[0].resources.session.unwrap();
            let file = attached(&owner, source, Bytes::from_static(b"frozen for later approval")).await;
            let first = job.start(input("plain", Some((source, file.clone())))).await.unwrap();
            assert!(matches!(first.targets[0].status, TargetStatus::Started));
            assert!(matches!(first.targets[1].status, TargetStatus::NeedsApproval(_)));
            approve(&owner, project.id, true).await;
            let arrival = bridge.hold(Method::CreateSession).await;
            {
                let continuing = job.continue_target(prepared.targets[1].index);
                tokio::pin!(continuing);
                tokio::select! {
                    reply = &mut continuing => panic!("held continued create completed {reply:?}"),
                    held = arrival => held.unwrap(),
                }
                // This method future is deliberately dropped. Its owned worker
                // must keep the admitted phase, not reconstruct old approval.
            }
            match outcome {
                Outcome::CompleteAfterDroppedReply => {
                    bridge.release().await;
                    let finished = changed(&job, |snapshot| matches!(snapshot.targets[1].status, TargetStatus::Started)).await;
                    let session = finished.targets[1].resources.session.unwrap();
                    let history = published_turns(&owner, session).await;
                    let vessel_proto::ContentBlock::Attachment { attachment } = &history[0][1] else { panic!("file"); };
                    assert_ne!(attachment.id, file.id);
                    assert_eq!(owner.read_attachment(session, attachment.id, 0).await.unwrap().data,
                        Bytes::from_static(b"frozen for later approval"));
                    assert!(matches!(job.continue_target(prepared.targets[1].index).await, Err(JobError::CannotContinue)));
                    assert_eq!(bridge.count(Method::PromptSession).await, 2);
                }
                Outcome::CancelCreate => {
                    let stopped = job.cancel().await.unwrap();
                    let target = &stopped.targets[1];
                    assert!(matches!(&target.status, TargetStatus::Uncertain(failure)
                        if failure.phase == Phase::SessionCreate && failure.kind == FailureKind::Cancelled));
                    assert!(target.resources.worktree.is_some());
                    assert!(target.resources.session.is_none(), "lost reply leaves identity unknown");
                    assert!(matches!(job.continue_target(target.index).await, Err(JobError::Closed)));
                    assert_eq!(bridge.count(Method::PromptSession).await, 1);
                }
            }
            assert_eq!(published_turns(&owner, source).await.len(), 1);
            assert_eq!(bridge.count(Method::CreateSession).await, 2);
            assert_eq!(bridge.count(Method::CreateWorktree).await, 3, "only an explicit known-no-effect approval retry");
            bridge.stop().await;
        }
        node.stop().await.unwrap();
    }).await;
}

/// The fake `plain` turn completes; its prompt ACK alone is not a log barrier.
/// Observe its end so the earlier `TurnStarted` is already in `next_seq` when
/// the ordinary history read captures its durable prefix. This also avoids
/// depending on the info update of the very record that woke a subscriber.
async fn published_turns(
    client: &NodeClient,
    session: SessionId,
) -> Vec<Vec<vessel_proto::ContentBlock>> {
    step("acknowledged comparison turn reaches the log", async {
        let mut stream = client.follow(session, EventSeq::ZERO).await.unwrap();
        read_until(&mut stream, &mut Vec::new(), |record| {
            matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { .. }
                }
            )
        })
        .await;
        turns(client, session).await
    })
    .await
}
