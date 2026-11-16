//! Synthetic assignment exercises the future source-new/importer-old branch.
//! Real transfer ownership is enforced even while the wire assignment stays None.

use super::tests::{exported, info, versions};
use super::*;
use crate::transfers::{Holder, IDLE, TransferError, Transfers};
use std::sync::Arc;
use std::time::Duration;

const CLEANUP_TEST_BUDGET: Duration = Duration::from_secs(30);
use tokio::sync::oneshot;
use vessel_proto::{DeviceId, NodeId, OriginBy, SessionListing};

fn holder() -> Holder {
    Holder::Delegated {
        device: DeviceId::random(),
        node: NodeId::random(),
        by: OriginBy::Owner,
    }
}

fn opened(transfers: &Transfers, holder: Holder) -> TransferId {
    let staged = transfers.stage(false).unwrap();
    transfers
        .commit(staged, holder, std::iter::empty())
        .unwrap()
}

fn returned(transfer: TransferId) -> Forwarded {
    let mut current = info();
    current.listing = SessionListing::Background;
    let mut value = exported(current);
    let Reply::ExportSession(manifest) = value.reply.as_mut() else {
        panic!("expected export");
    };
    manifest.transfer = transfer;
    value
}

fn refused(result: Result<(), ServiceError>, old: ProtocolVersion, assigned: ProtocolVersion) {
    let error = result.unwrap_err();
    assert_eq!(error.to_wire().code, vessel_proto::ErrorCode::Unsupported);
    let ServiceError::MetadataProtocol(actual) = error else {
        panic!("expected typed metadata refusal");
    };
    assert_eq!(
        actual,
        CallError::NotInProtocol {
            method: ExportSession::METHOD,
            needs: assigned,
            negotiated: old,
        }
    );
}

#[tokio::test]
async fn rejected_nested_exports_close_the_exact_transfer_with_its_original_holder() {
    vessel_testkit::within(CLEANUP_TEST_BUDGET, async {
        let temp = tempfile::tempdir().unwrap();
        let transfers = Arc::new(Transfers::new(temp.path().to_owned(), IDLE));
        let (old, new) = versions();
        let owner = holder();
        let unrelated = opened(&transfers, owner);
        let tasks = TaskTracker::new();
        for wrap in [Reply::Forward, Reply::Relay, Reply::Deliver] {
            let transfer = opened(&transfers, owner);
            assert!(matches!(transfers.close(Holder::Owner, transfer).await,
            Err(TransferError::Unknown(id)) if id == transfer));
            let value = Forwarded {
                reply: Box::new(wrap(returned(transfer))),
            };
            let source = Arc::clone(&transfers);
            refused(
                settle_at(&value, Some(new), old, &tasks, move |id| async move {
                    assert_eq!(id, transfer);
                    source
                        .close(owner, id)
                        .await
                        .map_err(|error| crate::transfers::wire(&error))
                })
                .await,
                old,
                new,
            );
            assert!(!temp.path().join(transfer.to_string()).exists());
            assert!(matches!(transfers.close(owner, transfer).await,
            Err(TransferError::Unknown(id)) if id == transfer));
        }
        assert!(temp.path().join(unrelated.to_string()).exists());
        transfers.close(owner, unrelated).await.unwrap();
        assert!(tasks.is_empty());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn aborting_the_caller_during_a_held_close_keeps_cleanup_tracked_until_settled() {
    vessel_testkit::within(CLEANUP_TEST_BUDGET, async {
        let temp = tempfile::tempdir().unwrap();
        let transfers = Arc::new(Transfers::new(temp.path().to_owned(), IDLE));
        let owner = holder();
        let transfer = opened(&transfers, owner);
        let value = returned(transfer);
        let tasks = TaskTracker::new();
        let tracked = tasks.clone();
        let source = Arc::clone(&transfers);
        let (entered, entry) = oneshot::channel();
        let (release, held) = oneshot::channel();
        let (old, new) = versions();
        let caller = tokio::spawn(async move {
            settle_at(&value, Some(new), old, &tracked, move |id| async move {
                assert_eq!(id, transfer);
                entered.send(()).unwrap();
                held.await.unwrap();
                source
                    .close(owner, id)
                    .await
                    .map_err(|error| crate::transfers::wire(&error))
            })
            .await
        });
        entry.await.unwrap();
        assert_eq!(tasks.len(), 1);
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        assert_eq!(tasks.len(), 1, "the only close must outlive its caller");
        assert!(temp.path().join(transfer.to_string()).exists());
        release.send(()).unwrap();
        assert!(tasks.close());
        tasks.wait().await;
        assert!(!temp.path().join(transfer.to_string()).exists());
        assert!(matches!(transfers.close(owner, transfer).await,
        Err(TransferError::Unknown(id)) if id == transfer));
    })
    .await
    .unwrap();
}

#[tokio::test(start_paused = true)]
async fn uncertain_close_is_bounded_by_existing_expiry_and_keeps_the_metadata_refusal() {
    let tasks = TaskTracker::new();
    let tracked = tasks.clone();
    let transfer = TransferId::random();
    let value = returned(transfer);
    let (old, new) = versions();
    let (entered, entry) = oneshot::channel();
    let caller = tokio::spawn(async move {
        settle_at(&value, Some(new), old, &tracked, move |id| async move {
            assert_eq!(id, transfer);
            entered.send(()).unwrap();
            std::future::pending().await
        })
        .await
    });
    entry.await.unwrap();
    assert_eq!(tasks.len(), 1);
    tokio::time::advance(IDLE).await;
    refused(
        tokio::time::timeout(IDLE, caller).await.unwrap().unwrap(),
        old,
        new,
    );
    assert!(tasks.is_empty());
    refused(
        settle_at(
            &returned(transfer),
            Some(new),
            old,
            &tasks,
            move |id| async move {
                assert_eq!(id, transfer);
                Err(WireError::new(
                    vessel_proto::ErrorCode::Forbidden,
                    "fixture scope refusal",
                ))
            },
        )
        .await,
        old,
        new,
    );
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn readable_default_and_nonexport_replies_never_send_a_close() {
    vessel_testkit::within(CLEANUP_TEST_BUDGET, async {
        let tasks = TaskTracker::new();
        let (old, new) = versions();
        for (value, assigned, peer) in [
            (exported(info()), None, old),
            (returned(TransferId::random()), Some(new), new),
            (
                Forwarded {
                    reply: Box::new(Reply::GetSession(info())),
                },
                None,
                old,
            ),
        ] {
            settle_at(&value, assigned, peer, &tasks, |_| async {
                panic!("a reply retained by its caller must retain its transfer");
            })
            .await
            .unwrap();
        }
        assert!(tasks.is_empty());
    })
    .await
    .unwrap();
}
