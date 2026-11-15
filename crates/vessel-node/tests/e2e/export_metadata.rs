//! An unreadable source export must refuse before provider history is inspected.

use crate::support::{Fixture, structured_spec, within};
use vessel_proto::{
    ErrorCode, ExportSession, Request as _, SessionId, SessionInfo, SessionListing,
};
use vessel_rpc::CallError;
use vessel_store::{Store, StoreConfig};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn source_export_refuses_unreadable_metadata_before_provider_history_work() {
    within(async {
        let fixture = Fixture::new();
        let dir = fixture.data_dir();
        dir.prepare().unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let plain = || {
            SessionInfo::new(
                SessionId::random(),
                structured_spec(&fixture, None),
                chrono::DateTime::UNIX_EPOCH,
            )
        };
        let mut background = plain();
        background.listing = SessionListing::Background;
        let mut labelled = plain();
        labelled.labels = serde_json::from_str(r#"{"env":"prod"}"#).unwrap();
        let originals = [background, labelled];
        for info in &originals {
            store.create_session(info.clone()).await.unwrap();
        }
        store.close().await;
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        for info in &originals {
            // No provider conversation exists: the metadata refusal must win
            // over that later export error, without reading/staging history.
            assert_eq!(info.harness_session, None);
            let result = client
                .call(ExportSession {
                    session: info.id,
                    force: false,
                })
                .await;
            let Err(CallError::Remote(error)) = result else {
                panic!("expected the source metadata refusal");
            };
            assert_eq!(error.code, ErrorCode::Unsupported);
            assert_eq!(
                error.message,
                CallError::UnassignedProtocol {
                    method: ExportSession::METHOD
                }
                .to_string()
            );
        }
        node.stop().await.unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        for original in originals {
            let kept = store.session(original.id).await.unwrap().unwrap();
            assert_eq!(kept.labels, original.labels);
            assert_eq!(kept.listing, original.listing);
        }
        store.close().await;
    })
    .await;
}
