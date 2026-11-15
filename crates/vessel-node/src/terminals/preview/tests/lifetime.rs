//! Synthetic observation time proves freshness policy without a network probe.

use super::*;
use tokio::time::Instant;

#[tokio::test(start_paused = true)]
async fn activity_does_not_refresh_but_complete_same_binding_evidence_does() {
    let rig = Rig::new(origin());
    let lease = rig.lease().await;
    let step = FRESH_FOR / 2;
    tokio::time::advance(step).await;
    lease.check(&rig.context, rig.origin).await.unwrap();
    rig.control.refresh(Instant::now());
    tokio::time::advance(step).await;
    lease.check(&rig.context, rig.origin).await.unwrap();
    tokio::time::advance(step).await;
    assert!(matches!(
        lease.check(&rig.context, rig.origin).await,
        Err(PreviewError::Expired | PreviewError::Revoked)
    ));
    lease.revoked().await;
    let _receipt = lease.revoke_and_drain().await;
}

#[tokio::test(start_paused = true)]
async fn future_observations_and_missing_sources_refuse_without_renewing() {
    let rig = Rig::new(origin());
    rig.control
        .refresh(Instant::now().checked_add(FRESH_FOR).unwrap());
    assert!(matches!(
        rig.service.admit(rig.context.clone(), rig.origin).await,
        Err(PreviewError::Clock)
    ));
    for reason in [
        MissingPreviewProof::CanonicalAuthority,
        MissingPreviewProof::RunIdentity,
        MissingPreviewProof::SocketPeer,
        MissingPreviewProof::Scheme,
        MissingPreviewProof::Platform,
    ] {
        let rig = Rig::new(origin());
        let lease = rig.lease().await;
        rig.control.missing(reason);
        assert!(lease.check(&rig.context, rig.origin).await.is_err());
        lease.revoked().await;
        let _receipt = lease.revoke_and_drain().await;
        assert!(!reason.to_string().is_empty());
    }
}

#[tokio::test]
async fn same_pid_new_run_and_each_connection_close_revoke_old_authority() {
    let rig = Rig::new(origin());
    let lease = rig.lease().await;
    let replacement = rig.control.replace_run();
    assert_eq!(
        replacement.observer().pid(),
        rig.control.run.observer().pid()
    );
    assert!(matches!(
        lease.check(&rig.context, rig.origin).await,
        Err(PreviewError::Changed | PreviewError::Revoked)
    ));
    lease.revoked().await;
    let _receipt = lease.revoke_and_drain().await;

    let rig = Rig::new(origin());
    let lease = rig.lease().await;
    rig.context.transport_closed.cancel();
    lease.revoked().await;
    assert!(lease.check(&rig.context, rig.origin).await.is_err());
    let _receipt = lease.revoke_and_drain().await;

    let rig = Rig::new(origin());
    let lease = rig.lease().await;
    drop(rig._connection);
    lease.revoked().await;
    let _receipt = lease.revoke_and_drain().await;
}

#[tokio::test]
async fn run_and_node_revocation_wake_without_a_new_command() {
    let rig = Rig::new(origin());
    let lease = rig.lease().await;
    rig.control.run.revoke();
    lease.revoked().await;
    let _receipt = lease.revoke_and_drain().await;
    let rig = Rig::new(origin());
    let lease = rig.lease().await;
    rig.service.runtime.shutdown.cancel();
    lease.revoked().await;
    let _receipt = lease.revoke_and_drain().await;
}

#[tokio::test]
async fn canonical_witness_loss_cannot_be_repaired_by_identical_metadata() {
    let rig = Rig::new(origin());
    let lease = rig.lease().await;
    rig.control.replace_authority();
    assert!(matches!(
        lease.check(&rig.context, rig.origin).await,
        Err(PreviewError::Changed | PreviewError::Revoked)
    ));
    lease.revoked().await;
    let _receipt = lease.revoke_and_drain().await;
}
