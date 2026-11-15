//! Real store/runtime discovery and inheritance use current effective metadata.

use super::*;
use chrono::DateTime;
use std::time::Duration;
use vessel_proto::{EventRecord, EventSeq, SessionEvent, SessionListing, SessionSelection};

async fn stored(
    fixture: &Fixture,
    listing: SessionListing,
    labels: &str,
    at: DateTime<chrono::Utc>,
) -> SessionId {
    let spec = fixture
        .spec()
        .with_labels(serde_json::from_str(r#"{"phase":"creation"}"#).unwrap());
    let mut info = SessionInfo::new(SessionId::random(), spec, at);
    info.listing = listing;
    info.labels = serde_json::from_str(labels).unwrap();
    fixture.store.create_session(info.clone()).await.unwrap();
    fixture
        .store
        .append(
            info.id,
            vec![Arc::new(EventRecord {
                seq: EventSeq::ZERO,
                at,
                event: SessionEvent::Status {
                    status: SessionStatus::Exited { code: Some(0) },
                },
            })],
        )
        .await
        .unwrap();
    info.id
}

async fn catalogue() -> (Fixture, [SessionId; 4]) {
    let fixture = Fixture::new().await;
    let matching = r#"{"env":"prod","region":"west"}"#;
    let shown = stored(
        &fixture,
        SessionListing::Shown,
        matching,
        DateTime::from_timestamp_millis(1).unwrap(),
    )
    .await;
    let hidden = stored(
        &fixture,
        SessionListing::Background,
        matching,
        DateTime::from_timestamp_millis(2).unwrap(),
    )
    .await;
    let other = stored(
        &fixture,
        SessionListing::Shown,
        r#"{"env":"prod","region":"east"}"#,
        DateTime::from_timestamp_millis(3).unwrap(),
    )
    .await;
    let live = fixture
        .sessions
        .create(
            fixture.spec().with_listing(SessionListing::Background),
            LaunchOptions::default(),
            None,
        )
        .await
        .unwrap()
        .id;
    (fixture, [shown, hidden, other, live])
}

async fn selected_ids(sessions: &Sessions, selection: SessionSelection) -> Vec<SessionId> {
    sessions
        .selected(&selection)
        .await
        .unwrap()
        .into_iter()
        .map(|info| info.id)
        .collect()
}

const BUDGET: Duration = Duration::from_secs(60);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selection_preserves_the_canonical_order_and_exact_id_access() {
    vessel_testkit::within(BUDGET, async {
        let (fixture, [shown, hidden, other, live]) = catalogue().await;
        let canonical = fixture.sessions.list().await.unwrap();
        let canonical_ids: Vec<_> = canonical.iter().map(|info| info.id).collect();
        assert_eq!(canonical_ids.len(), 4);
        assert_eq!(canonical_ids.first(), Some(&live));
        let selected = selected_ids(&fixture.sessions, SessionSelection::shown()).await;
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().all(|id| [shown, other].contains(id)));
        let matching = r#"{"env":"prod","region":"west"}"#;
        let one = SessionSelection::shown().with_labels(serde_json::from_str(matching).unwrap());
        assert_eq!(selected_ids(&fixture.sessions, one).await, [shown]);
        let inclusive =
            SessionSelection::all().with_labels(serde_json::from_str(matching).unwrap());
        let expected: Vec<_> = canonical_ids
            .iter()
            .copied()
            .filter(|id| [shown, hidden].contains(id))
            .collect();
        assert_eq!(selected_ids(&fixture.sessions, inclusive).await, expected);
        assert_eq!(
            selected_ids(&fixture.sessions, SessionSelection::all()).await,
            canonical_ids
        );
        for id in [shown, hidden, other, live] {
            assert_eq!(fixture.sessions.get(id).await.unwrap().id, id);
        }
        assert_eq!(
            fixture
                .sessions
                .get(hidden)
                .await
                .unwrap()
                .spec
                .labels
                .summary(),
            "phase=creation"
        );
        fixture.stop().await;
    })
    .await
    .unwrap();
}

async fn inherited(
    sessions: &Sessions,
    mut spec: SessionSpec,
    parent: Option<SessionId>,
) -> Result<Option<SessionListing>, ServiceError> {
    sessions.inherit_listing(&mut spec, parent).await?;
    Ok(spec.listing)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn omitted_listing_inherits_durable_or_live_background_and_explicit_shown_wins() {
    vessel_testkit::within(BUDGET, async {
        let (fixture, [shown, saved, _, live]) = catalogue().await;
        for parent in [saved, live] {
            let omitted = inherited(&fixture.sessions, fixture.structured(), Some(parent))
                .await
                .unwrap();
            assert_eq!(omitted, Some(SessionListing::Background));
            let spec = fixture.structured().with_listing(SessionListing::Shown);
            assert_eq!(
                inherited(&fixture.sessions, spec, Some(parent))
                    .await
                    .unwrap(),
                Some(SessionListing::Shown)
            );
        }
        for parent in [None, Some(shown)] {
            let omitted = inherited(&fixture.sessions, fixture.structured(), parent)
                .await
                .unwrap();
            assert_eq!(omitted, None, "shown keeps the legacy wire path");
        }
        let missing = SessionId::random();
        assert!(
            matches!(inherited(&fixture.sessions, fixture.structured(), Some(missing)).await,
            Err(ServiceError::NotFound(id)) if id == missing)
        );
        fixture.stop().await;
    })
    .await
    .unwrap();
}
