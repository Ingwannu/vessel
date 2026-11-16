//! The usage client scope stays open while either source owner runs and
//! drains only after both have ended.
use super::{UsageClient, context, retire};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio_util::task::TaskTracker;
use vessel_relay::resolution::{
    FailureKind, HostName, ResolutionBudget, ResolutionHandle, ResolutionPort,
};

const PHASE: Duration = Duration::from_secs(20);

async fn admits(handle: &ResolutionHandle<UsageClient>) -> Result<(), FailureKind> {
    handle
        .resolve(&HostName::new("127.0.0.1").unwrap(), ResolutionPort(0))
        .await
        .map(|_found| ())
        .map_err(|error| error.kind())
}

#[tokio::test]
async fn the_client_scope_closes_only_after_both_source_owners_end() {
    tokio::time::timeout(PHASE, async {
        let budget = ResolutionBudget::default();
        let (_context, scope) = context(&budget);
        let handle = scope.as_ref().unwrap().handle();
        let owners = TaskTracker::new();
        let (end_limits, limits) = oneshot::channel::<()>();
        let (end_sources, sources) = oneshot::channel::<()>();
        drop(owners.spawn(async move {
            let _ended = limits.await;
        }));
        drop(owners.spawn(async move {
            let _ended = sources.await;
        }));
        let retiring = tokio::spawn(retire(owners.clone(), scope));
        tokio::task::yield_now().await;
        assert_eq!(admits(&handle).await, Ok(()));
        end_limits.send(()).unwrap();
        tokio::task::yield_now().await;
        assert!(!retiring.is_finished());
        assert_eq!(admits(&handle).await, Ok(()));
        end_sources.send(()).unwrap();
        retiring.await.unwrap();
        assert_eq!(admits(&handle).await, Err(FailureKind::Closed));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn retiring_without_a_scope_only_waits_for_the_owners() {
    tokio::time::timeout(PHASE, async {
        let owners = TaskTracker::new();
        let (end, ended) = oneshot::channel::<()>();
        drop(owners.spawn(async move {
            let _ended = ended.await;
        }));
        let retiring = tokio::spawn(retire(owners.clone(), None));
        tokio::task::yield_now().await;
        assert!(!retiring.is_finished());
        end.send(()).unwrap();
        retiring.await.unwrap();
        assert!(owners.is_closed() && owners.is_empty());
    })
    .await
    .unwrap();
}
