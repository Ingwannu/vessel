//! Stored-source changes cannot silently produce a replacement snapshot.
use super::faults::bridge::Bridge;
use super::*;
use crate::support::within;
use vessel_client::comparison::Phase;
use vessel_proto::Method;

enum Change {
    SameLengthMutation,
    Delete,
    DeleteEmpty,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn changed_or_missing_source_fails_remaining_copies_without_resending_success() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        for change in [
            Change::SameLengthMutation,
            Change::Delete,
            Change::DeleteEmpty,
        ] {
            let bridge = Bridge::open(&fixture, &node).await;
            let job = prepared(&bridge.client, project.id, &["fake-small", "fake-large"]).await;
            let source = job.snapshot().targets[0].resources.session.unwrap();
            let bytes = match change {
                Change::DeleteEmpty => b"".as_slice(),
                Change::SameLengthMutation | Change::Delete => b"original bytes".as_slice(),
            };
            let file = attached(&owner, source, Bytes::copy_from_slice(bytes)).await;
            let arrival = bridge.hold(Method::PromptSession).await;
            let sending = job.start(input("plain", Some((source, file.clone()))));
            tokio::pin!(sending);
            tokio::select! {
                reply = &mut sending => panic!("held first prompt completed {reply:?}"),
                held = arrival => held.unwrap(),
            }
            let path = fixture
                .data_dir()
                .attachments_dir()
                .join(source.to_string())
                .join(file.id.to_string())
                .join(file.name.as_str());
            match change {
                Change::SameLengthMutation => std::fs::write(path, b"ORIGINAL BYTES").unwrap(),
                Change::Delete | Change::DeleteEmpty => std::fs::remove_file(path).unwrap(),
            }
            bridge.release().await;
            let snapshot = sending.await.unwrap();
            assert!(matches!(snapshot.targets[0].status, TargetStatus::Started));
            assert!(
                matches!(&snapshot.targets[1].status, TargetStatus::Failed(failure)
                if failure.phase == Phase::AttachmentRead),
                "{snapshot:?}"
            );
            assert!(
                snapshot.targets[1]
                    .resources
                    .copy
                    .as_ref()
                    .unwrap()
                    .upload
                    .is_some()
            );
            // The source's turn is readable once it has ended (`turn_ended`):
            // read right after the job's snapshot it could still be on its
            // way (W11's run, 2026-11-15).
            turn_ended(&owner, source).await;
            assert_eq!(turns(&owner, source).await.len(), 1);
            assert!(
                turns(&owner, snapshot.targets[1].resources.session.unwrap())
                    .await
                    .is_empty()
            );
            assert_eq!(bridge.count(Method::PromptSession).await, 1);
            assert_eq!(bridge.count(Method::FinishUpload).await, 0);
            bridge.stop().await;
        }
        node.stop().await.unwrap();
    })
    .await;
}
