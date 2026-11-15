//! Capacity and shutdown are properties of admitted operations, not caller tasks.
use super::*;

#[tokio::test]
async fn the_single_owned_lane_bounds_pending_changes_without_losing_admitted_ones() {
    within(async {
        let f = Fixture::new(None).await;
        let (entered, release) = f.settings.engine.gates.arm_write();
        let settings = Arc::clone(&f.settings);
        let first = tokio::spawn(async move { settings.change(patch(idle())).await });
        step("first write", entered).await.unwrap();
        let mut waiting = Vec::new();
        for _ in 0..Settings::QUEUED_CHANGES {
            let mut next = Box::pin(f.settings.change(patch(ParkAfter::Never)));
            assert!(futures_util::poll!(next.as_mut()).is_pending());
            waiting.push(next);
        }
        assert!(matches!(
            f.settings.change(patch(ParkAfter::Never)).await,
            Err(SettingsError::Full)
        ));
        release.send(()).unwrap();
        assert_eq!(
            step("first completes", first)
                .await
                .unwrap()
                .unwrap()
                .park_after,
            idle()
        );
        for reply in waiting {
            assert_eq!(
                step("admitted queued change", reply)
                    .await
                    .unwrap()
                    .park_after,
                ParkAfter::Never
            );
        }
        assert_eq!(f.settings.current().await.park_after, ParkAfter::Never);
        assert_eq!(f.runtime.park_after(), None);
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn drain_closes_admission_but_waits_for_the_real_write_and_apply() {
    within(async {
        let f = Fixture::new(None).await;
        let (entered, release) = f.settings.engine.gates.arm_write();
        let settings = Arc::clone(&f.settings);
        let change = tokio::spawn(async move { settings.change(patch(idle())).await });
        step("write held for shutdown", entered).await.unwrap();
        let mut drain = Box::pin(f.settings.drain());
        assert!(futures_util::poll!(drain.as_mut()).is_pending());
        assert!(matches!(
            f.settings.change(patch(ParkAfter::Never)).await,
            Err(SettingsError::Closed)
        ));
        release.send(()).unwrap();
        step("drain admitted mutation", drain).await;
        assert_eq!(
            step("change result", change)
                .await
                .unwrap()
                .unwrap()
                .park_after,
            idle()
        );
        assert_eq!(f.runtime.park_after(), idle().duration());
        assert_eq!(f.settings.current().await.park_after, idle());
        f.finish().await;
    })
    .await;
}
