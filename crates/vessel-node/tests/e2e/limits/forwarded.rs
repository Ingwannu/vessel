//! The new methods obey the existing destination grant over a real fleet.
use super::*;
use crate::across::{home, member_granting};
use crate::secure::Relay;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn forwarded_cached_limits_need_read_but_refresh_and_policy_keep_their_scopes() {
    within(async {
        let relay = Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = home(&relay, sources.path()).await;
        let read = member_granting(
            &relay,
            &home,
            "quota-reader",
            vec![Scope::Sync],
            vec![Scope::Read],
        )
        .await;
        let admin = member_granting(
            &relay,
            &home,
            "quota-admin",
            vec![Scope::Sync],
            vec![Scope::Read, Scope::Operate, Scope::Admin],
        )
        .await;
        assert_eq!(
            home.owner
                .call_on(read.id(), query(LimitRefresh::Cached))
                .await
                .unwrap()
                .state,
            AccountLimitState::Off
        );
        assert_eq!(
            remote(
                home.owner
                    .call_on(read.id(), query(LimitRefresh::Refresh))
                    .await
            ),
            ErrorCode::Forbidden
        );
        assert_eq!(
            remote(
                home.owner
                    .call_on(
                        read.id(),
                        SetUsageLookup {
                            policy: ExternalLookup::On
                        }
                    )
                    .await
            ),
            ErrorCode::Forbidden
        );
        assert_eq!(
            home.owner
                .call_on(
                    admin.id(),
                    SetUsageLookup {
                        policy: ExternalLookup::On
                    }
                )
                .await
                .unwrap(),
            ExternalLookup::On
        );
        assert_eq!(
            home.owner
                .call_on(admin.id(), query(LimitRefresh::Cached))
                .await
                .unwrap()
                .state,
            AccountLimitState::NotRequested
        );
        // The local source is disabled in this fixture. Scope admission is
        // nevertheless demonstrated by the typed unsupported source result.
        assert_eq!(
            home.owner
                .call_on(admin.id(), query(LimitRefresh::Refresh))
                .await
                .unwrap()
                .state,
            AccountLimitState::Failed {
                reason: LimitFailure::Unsupported,
                retained: None,
                retry_at: None
            }
        );
        assert_eq!(
            home.owner
                .call_on(
                    admin.id(),
                    SetUsageLookup {
                        policy: ExternalLookup::Off
                    }
                )
                .await
                .unwrap(),
            ExternalLookup::Off
        );
        admin.node.stop().await.unwrap();
        read.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}
