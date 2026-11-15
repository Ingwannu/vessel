//! Real loopback bytes exercise the wrapper, with synthetic ownership only.

use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;

fn owned_stream<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send>(_stream: &T) {}

#[tokio::test]
async fn payload_waits_for_proof_and_revocation_wakes_pending_io() {
    vessel_testkit::within(TEST_WAIT, async {
        let (listener, origin) = listener().await;
        let rig = Rig::new(origin);
        let lease = rig.lease().await;
        rig.control.inspection.block();
        let task = tokio::spawn({
            let lease = lease.clone();
            let context = rig.context.clone();
            async move { lease.connect_verified(&context, origin).await }
        });
        let (mut peer, _) = listener.accept().await.unwrap();
        rig.control.inspection.entered.notified().await;
        assert!(matches!(peer.try_read(&mut [0]), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
        assert!(!task.is_finished());
        rig.control.inspection.release();
        let mut stream = task.await.unwrap().unwrap();
        owned_stream(&stream);
        stream.write_all(b"request").await.unwrap();
        stream.flush().await.unwrap();
        let mut bytes = [0; b"request".len()];
        peer.read_exact(&mut bytes).await.unwrap();
        assert_eq!(&bytes, b"request");
        {
            let mut buffer = [0];
            let mut reading = std::pin::pin!(stream.read(&mut buffer));
            pending(reading.as_mut()).await;
            rig.control.run.revoke();
            assert_eq!(reading.await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
        }
        assert_eq!(stream.write(b"late").await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
        assert_eq!(stream.flush().await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
        assert_eq!(stream.shutdown().await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
        let mut draining = std::pin::pin!(lease.revoke_and_drain());
        pending(draining.as_mut()).await;
        assert_eq!(rig.service.runtime.connections.available_permits(), MAX_PREVIEW_STREAMS - 1);
        drop(stream);
        let receipt = draining.await;
        assert!(receipt.belongs_to(&lease));
        assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
    }).await;
}

#[tokio::test]
async fn cancelled_caller_does_not_release_worker_capacity_or_finish_drain() {
    vessel_testkit::within(TEST_WAIT, async {
        let (listener, origin) = listener().await;
        let rig = Rig::new(origin);
        let lease = rig.lease().await;
        rig.control.inspection.block();
        let task = tokio::spawn({
            let lease = lease.clone();
            let context = rig.context.clone();
            async move { lease.connect_verified(&context, origin).await }
        });
        let (mut peer, _) = listener.accept().await.unwrap();
        rig.control.inspection.entered.notified().await;
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(
            rig.service.runtime.preparations.available_permits(),
            MAX_PREVIEW_PREPARATIONS - 1
        );
        assert_eq!(
            rig.service.runtime.connections.available_permits(),
            MAX_PREVIEW_STREAMS - 1
        );
        let mut draining = std::pin::pin!(lease.revoke_and_drain());
        pending(draining.as_mut()).await;
        rig.control.inspection.release();
        let receipt = draining.await;
        assert!(receipt.belongs_to(&lease));
        assert_eq!(
            rig.service.runtime.preparations.available_permits(),
            MAX_PREVIEW_PREPARATIONS
        );
        assert_eq!(
            rig.service.runtime.connections.available_permits(),
            MAX_PREVIEW_STREAMS
        );
        assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
        assert!(matches!(
            lease.connect_verified(&rig.context, origin).await,
            Err(PreviewError::Revoked)
        ));
    })
    .await;
}

#[tokio::test]
async fn operation_deadline_also_leaves_physical_inspection_owned() {
    vessel_testkit::within(TEST_WAIT, async {
        let (listener, origin) = listener().await;
        let rig = Rig::new(origin);
        let lease = rig.lease().await;
        rig.control.inspection.block();
        let task = tokio::spawn({
            let lease = lease.clone();
            let context = rig.context.clone();
            async move { lease.connect_verified(&context, origin).await }
        });
        let (_peer, _) = listener.accept().await.unwrap();
        rig.control.inspection.entered.notified().await;
        tokio::time::pause();
        tokio::time::advance(PREVIEW_OPERATION_TIMEOUT).await;
        assert!(matches!(
            task.await.unwrap(),
            Err(PreviewError::Deadline(_))
        ));
        tokio::time::resume();
        assert_eq!(
            rig.service.runtime.preparations.available_permits(),
            MAX_PREVIEW_PREPARATIONS - 1
        );
        let mut draining = std::pin::pin!(lease.revoke_and_drain());
        pending(draining.as_mut()).await;
        rig.control.inspection.release();
        let _receipt = draining.await;
        assert_eq!(
            rig.service.runtime.preparations.available_permits(),
            MAX_PREVIEW_PREPARATIONS
        );
    })
    .await;
}

#[tokio::test]
async fn stale_or_foreign_commands_do_not_dial_and_busy_refusals_preserve_causes() {
    vessel_testkit::within(TEST_WAIT, async {
        use std::error::Error as _;
        let (listener, origin) = listener().await;
        let rig = Rig::new(origin);
        let lease = rig.lease().await;
        let foreign = context(&ConnectionLife::default());
        assert!(matches!(
            lease.connect_verified(&foreign, origin).await,
            Err(PreviewError::Scope)
        ));
        for (pool, budget) in [
            (&rig.service.runtime.connections, PreviewBudget::Connections),
            (&lease.0.connections, PreviewBudget::LeaseConnections),
            (
                &rig.service.runtime.preparations,
                PreviewBudget::Preparation,
            ),
        ] {
            let all = Arc::clone(pool)
                .try_acquire_many_owned(u32::try_from(budget.limit()).unwrap())
                .unwrap();
            let result = lease.connect_verified(&rig.context, origin).await;
            let Err(error @ PreviewError::Busy { .. }) = result else {
                panic!("connection capacity was bypassed")
            };
            assert!(error.source().is_some());
            assert!(error.to_string().contains(&budget.limit().to_string()));
            drop(all);
        }
        pending(std::pin::pin!(listener.accept())).await;
        let _receipt = lease.revoke_and_drain().await;
    })
    .await;
}
