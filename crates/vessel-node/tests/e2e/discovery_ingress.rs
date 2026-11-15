//! Dynamic import must validate durable discovery metadata before export or placement.

use crate::support::{Fixture, structured_spec, within};
use vessel_proto::{ErrorCode, ImportSession, SessionId, SessionInfo, SessionListing, SessionRef};
use vessel_rpc::CallError;
use vessel_store::{Store, StoreConfig};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hidden_listing_is_kept_in_storage_but_refused_before_starting_import_work() {
    within(async {
        let fixture = Fixture::new();
        let dir = fixture.data_dir();
        dir.prepare().unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let mut background = SessionInfo::new(
            SessionId::random(),
            structured_spec(&fixture, None),
            chrono::DateTime::UNIX_EPOCH,
        );
        // The effective classification can be an immutable mirror even when
        // the original creation spec omitted an independent override.
        background.listing = SessionListing::Background;
        let explicit_shown = SessionInfo::new(
            SessionId::random(),
            structured_spec(&fixture, None).with_listing(SessionListing::Shown),
            chrono::DateTime::UNIX_EPOCH,
        );
        for info in [&background, &explicit_shown] {
            store.create_session(info.clone()).await.unwrap();
        }
        store.close().await;
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        assert_eq!(client.protocol(), vessel_proto::PROTOCOL_VERSION);
        // Legacy null still returns the full collection; pending reply metadata
        // cannot silently turn canonical discovery into a filtered collection.
        assert_eq!(client.sessions().await.unwrap().len(), 2);
        for info in [&background, &explicit_shown] {
            let hidden = client.session(info.id).await.unwrap();
            assert_eq!(hidden.listing, SessionListing::Shown);
            assert_eq!(hidden.spec.listing, None);
            let destination = fixture.user_home().join(info.id.to_string());
            let result = client
                .import_session(ImportSession {
                    from: SessionRef {
                        node: node.info().id,
                        session: info.id,
                    },
                    profile: None,
                    cwd: Some(destination.clone()),
                    start: true,
                })
                .await;
            let Err(CallError::Remote(error)) = result else {
                panic!("expected the dynamic source metadata refusal");
            };
            assert_eq!(error.code, ErrorCode::Unsupported);
            assert_eq!(
                error.message,
                "session listing has no assigned protocol version"
            );
            assert!(!destination.exists());
            assert_eq!(client.sessions().await.unwrap().len(), 2);
        }
        node.stop().await.unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        for original in [background, explicit_shown] {
            let kept = store.session(original.id).await.unwrap().unwrap();
            assert_eq!(kept.listing, original.listing);
            assert_eq!(kept.spec.listing, original.spec.listing);
        }
        store.close().await;
    })
    .await;
}
