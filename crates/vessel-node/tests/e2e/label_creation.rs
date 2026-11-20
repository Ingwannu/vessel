//! Creation labels remain current metadata across node restart.
//! Exact-id access and an unfiltered list keep the same session set.

use crate::support::{Fixture, structured_spec, within};
use vessel_proto::{ListSessions, SessionLabels};

fn labels(text: &str) -> SessionLabels {
    serde_json::from_str(text).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn creation_labels_are_current_metadata_and_survive_restart() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let requested = labels(r#"{"env":"prod","tier":"a"}"#);
        let first = client
            .create_session(structured_spec(&fixture, None).with_labels(requested.clone()))
            .await
            .unwrap();
        let second = client
            .create_session(
                structured_spec(&fixture, None).with_labels(labels(r#"{"env":"prod","tier":"b"}"#)),
            )
            .await
            .unwrap();
        let filtered = client
            .call(ListSessions::matching(requested.clone()))
            .await
            .unwrap();
        assert_eq!(
            filtered.iter().map(|info| info.id).collect::<Vec<_>>(),
            [first.id]
        );
        assert_eq!(client.sessions().await.unwrap().len(), 2);
        assert_eq!(
            client.session(second.id).await.unwrap().labels,
            labels(r#"{"env":"prod","tier":"b"}"#)
        );
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let restored = client.session(first.id).await.unwrap();
        assert_eq!(restored.labels, requested);
        assert_eq!(restored.spec.labels, requested);
        let filtered = client
            .call(ListSessions::matching(labels(
                r#"{"env":"prod","tier":"b"}"#,
            )))
            .await
            .unwrap();
        assert_eq!(
            filtered.iter().map(|info| info.id).collect::<Vec<_>>(),
            [second.id]
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn creation_labels_are_not_silently_discarded_by_the_side_shortcut() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut spec = structured_spec(&fixture, None).with_labels(labels(r#"{"env":"prod"}"#));
        spec.side_question_of = Some(vessel_proto::SessionId::random());
        let error = client.create_session(spec).await.unwrap_err();
        assert_eq!(error.code(), Some(vessel_proto::ErrorCode::InvalidRequest));
        assert!(error.to_string().contains("creation labels"));
        assert!(client.sessions().await.unwrap().is_empty());
        node.stop().await.unwrap();
    })
    .await;
}
