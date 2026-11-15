//! A typed frame bridge withholds real node acknowledgements after their effects.
pub(super) mod bridge;
mod deadlines;
mod drop_owner;
use super::*;
use crate::support::within;
use bridge::Bridge;
use vessel_client::comparison::{FailureKind, JobError, Phase};
use vessel_proto::Method;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lost_upload_acknowledgements_retain_phase_ids_offsets_and_never_prompt() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        for (method, phase) in [
            (Method::BeginUpload, Phase::UploadBegin),
            (Method::WriteUpload, Phase::UploadWrite),
            (Method::FinishUpload, Phase::UploadFinish),
        ] {
            let bridge = Bridge::open(&fixture, &node).await;
            let job = prepared(&bridge.client, project.id, &["fake-small", "fake-large"]).await;
            let source = job.snapshot().targets[0].resources.session.unwrap();
            let file = attached(&owner, source, Bytes::from_static(b"frozen common bytes")).await;
            let arrival = bridge.hold(method).await;
            let send = job.start(input("plain", Some((source, file))));
            tokio::pin!(send);
            tokio::select! {
                reply = &mut send => panic!("lost reply completed: {reply:?}"),
                held = arrival => held.unwrap(),
            }
            let before = bridge.count(method).await;
            let final_report = job.cancel().await.unwrap();
            let target = &final_report.targets[1];
            assert!(matches!(&target.status, TargetStatus::Uncertain(failure)
                if failure.phase == phase && failure.kind == FailureKind::Cancelled));
            let trail = target.resources.copy.as_ref().unwrap();
            assert_eq!(trail.upload.is_none(), method == Method::BeginUpload);
            assert_eq!(
                trail.acknowledged,
                if method == Method::FinishUpload {
                    19
                } else {
                    0
                }
            );
            if method == Method::WriteUpload {
                assert_eq!(trail.attempted_bytes, 19);
            }
            assert!(
                turns(&owner, target.resources.session.unwrap())
                    .await
                    .is_empty()
            );
            assert!(matches!(
                job.continue_target(target.index).await,
                Err(JobError::Closed)
            ));
            assert_eq!(bridge.count(method).await, before);
            assert_eq!(turns(&owner, source).await.len(), 1);
            bridge.stop().await;
        }
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_a_prompt_reply_future_keeps_the_owner_and_sends_once() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        let bridge = Bridge::open(&fixture, &node).await;
        let job = prepared(&bridge.client, project.id, &["fake-small"]).await;
        let session = job.snapshot().targets[0].resources.session.unwrap();
        let arrival = bridge.hold(Method::PromptSession).await;
        {
            let sending = job.start(input("plain", None));
            tokio::pin!(sending);
            tokio::select! {
                reply = &mut sending => panic!("held reply completed: {reply:?}"),
                held = arrival => held.unwrap(),
            }
        }
        assert!(job.snapshot().input_bound);
        assert_eq!(turns(&owner, session).await.len(), 1);
        bridge.release().await;
        let finished = changed(&job, |snapshot| {
            matches!(snapshot.targets[0].status, TargetStatus::Started)
        })
        .await;
        assert!(matches!(
            job.start(input("retry", None)).await,
            Err(JobError::AlreadyBound)
        ));
        assert_eq!(bridge.count(Method::PromptSession).await, 1);
        assert_eq!(
            turns(&owner, finished.targets[0].resources.session.unwrap())
                .await
                .len(),
            1
        );
        bridge.stop().await;
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_finish_metadata_mismatch_retains_the_actual_stored_attachment_id() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        let bridge = Bridge::open(&fixture, &node).await;
        let job = prepared(&bridge.client, project.id, &["fake-small", "fake-large"]).await;
        let source = job.snapshot().targets[0].resources.session.unwrap();
        let file = attached(&owner, source, Bytes::from_static(b"stored bytes")).await;
        let arrival = bridge.hold(Method::FinishUpload).await;
        let sending = job.start(input("plain", Some((source, file))));
        tokio::pin!(sending);
        tokio::select! {
            reply = &mut sending => panic!("held finish completed {reply:?}"),
            held = arrival => held.unwrap(),
        }
        bridge
            .release_renamed("wrong-name.txt".parse().unwrap())
            .await;
        let report = sending.await.unwrap();
        let target = &report.targets[1];
        assert!(matches!(&target.status, TargetStatus::Failed(failure)
            if failure.phase == Phase::UploadFinish && failure.kind == FailureKind::Copy));
        let progress = target.resources.copy.as_ref().unwrap();
        assert!(progress.upload.is_some());
        assert_eq!(progress.acknowledged, 12);
        let stored = progress.finished.unwrap();
        assert_eq!(
            owner
                .read_attachment(target.resources.session.unwrap(), stored, 0)
                .await
                .unwrap()
                .data,
            Bytes::from_static(b"stored bytes")
        );
        assert!(
            turns(&owner, target.resources.session.unwrap())
                .await
                .is_empty()
        );
        assert_eq!(turns(&owner, source).await.len(), 1);
        assert_eq!(bridge.count(Method::FinishUpload).await, 1);
        assert_eq!(bridge.count(Method::PromptSession).await, 1);
        bridge.stop().await;
        node.stop().await.unwrap();
    })
    .await;
}
