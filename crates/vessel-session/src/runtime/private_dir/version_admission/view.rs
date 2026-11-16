//! Separate node-only admission from the combined hard-drain observation.

use super::*;
#[derive(Clone, Copy)]
enum DrainOrder {
    AfterNodeRefusal,
    BeforeLookupRelease,
}

async fn public_view_node_close(order: DrainOrder) {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async move {
            let (mut fixture, profile, audit) = native_fixture(None);
            let mut barrier = hold(&mut fixture);
            let node_stop = CancellationToken::new();
            fixture.runtime = fixture.runtime.with_admission(node_stop.clone());
            let work = fixture.work();
            let runtime = Arc::new(fixture.runtime);
            let slots = Arc::new(Semaphore::new(NonZeroUsize::MIN.get()));
            let permit = Arc::clone(&slots).try_acquire_owned().unwrap();
            let mut preparing = {
                let runtime = Arc::clone(&runtime);
                tokio::spawn(async move {
                    runtime
                        .with_preparation(PreparationGate::new(
                            CancellationToken::new(),
                            |_effect, live: PreparationCancellation<'_>| match live.check() {
                                Ok(()) => PreparationAdmission::Admitted,
                                Err(reason) => PreparationAdmission::Refused(reason),
                            },
                        ))
                        .probe_with_choices_owned(
                            HarnessKind::Codex.into(),
                            Some(profile),
                            None,
                            work,
                            crate::ProbeChoices::default(),
                            permit,
                        )
                        .await
                })
            };
            let held = tokio::time::timeout(STEP, barrier.arrived())
                .await
                .unwrap()
                .unwrap();
            node_stop.cancel();
            assert_eq!(slots.available_permits(), 0);
            pending(std::pin::Pin::new(&mut preparing)).await;
            let mut draining = match order {
                DrainOrder::AfterNodeRefusal => None,
                DrainOrder::BeforeLookupRelease => Some(Box::pin(runtime.shutdown())),
            };
            if let Some(draining) = draining.as_mut() {
                pending(draining.as_mut()).await;
            }
            drop(held);
            let result = tokio::time::timeout(STEP, preparing)
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(
                result,
                Err(crate::SessionError::Preparation(
                    PreparationRefusal::Cancelled
                ))
            ));
            // In AfterNodeRefusal no runtime shutdown future has even been
            // created: only the node source can explain this refusal/witness.
            assert!(!runtime.version_control.has_started());
            assert!(!audit.exists());
            match draining {
                Some(draining) => tokio::time::timeout(STEP, draining).await.unwrap(),
                None => tokio::time::timeout(STEP, runtime.shutdown())
                    .await
                    .unwrap(),
            }
            assert_eq!(slots.available_permits(), NonZeroUsize::MIN.get());
            assert!(runtime.pids().is_empty());
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_view_node_only_close_refuses_before_version_without_runtime_shutdown() {
    public_view_node_close(DrainOrder::AfterNodeRefusal).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_view_node_close_retains_lookup_capacity_through_runtime_drain() {
    public_view_node_close(DrainOrder::BeforeLookupRelease).await;
}
