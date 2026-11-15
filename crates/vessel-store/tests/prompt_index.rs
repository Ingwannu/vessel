//! Durable snapshot metadata remains paged across appends and rejects rewinds.

use vessel_proto::prompt_index::{ListSessionPrompts, PromptIndexLimit};
use vessel_proto::{EventSeq, SessionEvent, SessionId};
use vessel_store::{StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(dead_code, reason = "shared store builders; this test uses a subset")]
mod fixtures;
#[cfg(test)]
#[path = "support/messages.rs"]
#[expect(dead_code, reason = "shared search builders; this test uses a subset")]
mod messages;

fn request(session: SessionId) -> ListSessionPrompts {
    ListSessionPrompts {
        session,
        after: None,
        epoch: None,
        limit: PromptIndexLimit::new(1).unwrap(),
    }
}

#[tokio::test]
async fn prompt_index_reads_one_snapshot_and_survives_reopen_without_a_new_schema() {
    let (_temp, dir) = fixtures::data_dir();
    let store = fixtures::open(&dir, StoreConfig::default()).await;
    let session = messages::started(&store, 0).await;
    assert!(
        store
            .prompt_index(request(session))
            .await
            .unwrap()
            .entries()
            .is_empty()
    );
    store
        .append(
            session,
            vec![messages::prompt(1, "first"), messages::prompt(2, "second")],
        )
        .await
        .unwrap();
    let first = store.prompt_index(request(session)).await.unwrap();
    assert_eq!(first.entries().first().unwrap().preview.as_str(), "first");
    assert_eq!(first.next(), Some(EventSeq::new(1)));
    store
        .append(session, vec![messages::prompt(3, "third")])
        .await
        .unwrap();
    let mut next = request(session);
    next.after = first.next();
    next.epoch = Some(first.epoch());
    let second = store.prompt_index(next.clone()).await.unwrap();
    assert_eq!(second.entries().first().unwrap().preview.as_str(), "second");
    assert_eq!(first.epoch(), second.epoch());
    store
        .append(
            session,
            vec![fixtures::record(
                4,
                SessionEvent::Rewound {
                    after: EventSeq::new(1),
                    harness_session: "fake-history".parse().unwrap(),
                    files_changed: 0,
                },
            )],
        )
        .await
        .unwrap();
    assert!(matches!(
        store.prompt_index(next).await,
        Err(StoreError::PromptIndexChanged)
    ));
    let visible = store.prompt_index(request(session)).await.unwrap();
    assert_eq!(visible.entries().len(), 1);
    assert_eq!(visible.next(), None);
    drop(store);
    let reopened = fixtures::open(&dir, StoreConfig::default()).await;
    assert_eq!(
        reopened.prompt_index(request(session)).await.unwrap(),
        visible
    );
    assert!(matches!(
        reopened.prompt_index(request(SessionId::random())).await,
        Err(StoreError::UnknownSession(_))
    ));
}

#[tokio::test]
async fn prompt_index_retention_invalidates_a_continuation_and_never_returns_pruned_prompts() {
    let (_temp, dir) = fixtures::data_dir();
    let config = StoreConfig {
        retention: vessel_store::Retention {
            max_session_bytes: vessel_store::ByteCount::new(512),
            ..Default::default()
        },
        ..Default::default()
    };
    let store = fixtures::open(&dir, config).await;
    let session = messages::started(&store, 0).await;
    store
        .append(
            session,
            vec![messages::prompt(1, "first"), messages::prompt(2, "second")],
        )
        .await
        .unwrap();
    let first = store.prompt_index(request(session)).await.unwrap();
    assert_eq!(first.next(), Some(EventSeq::new(1)));
    store
        .append(session, vec![messages::prompt(3, &"x".repeat(4_096))])
        .await
        .unwrap();
    let mut continuation = request(session);
    continuation.after = first.next();
    continuation.epoch = Some(first.epoch());
    assert!(matches!(
        store.prompt_index(continuation).await,
        Err(StoreError::PromptIndexChanged)
    ));
    let retained = store.prompt_index(request(session)).await.unwrap();
    assert!(retained.epoch().retained_from() > first.epoch().retained_from());
    assert_eq!(retained.entries().len(), 1);
    assert_eq!(retained.entries()[0].seq, EventSeq::new(3));
    store.close().await;
}

/// Explicit timing evidence; no time threshold turns machine load into a test failure.
#[tokio::test]
#[ignore = "measurement of repeated snapshot paging; run explicitly with --ignored --nocapture"]
async fn prompt_index_measure_metadata_pages_over_twenty_thousand_records() {
    use std::io::Write as _;
    use std::time::Instant;
    const TURNS: u64 = 10_000;
    const CHUNK_BYTES: usize = 1_024;
    let (_temp, dir) = fixtures::data_dir();
    let store = fixtures::open(&dir, StoreConfig::default()).await;
    let session = messages::started(&store, 0).await;
    let reply = "r".repeat(CHUNK_BYTES);
    let records = (0..TURNS)
        .flat_map(|turn| {
            let seq = turn.saturating_mul(2).saturating_add(1);
            [
                messages::prompt(seq, "한글 prompt navigation measurement"),
                messages::chunk(seq.saturating_add(1), &reply),
            ]
        })
        .collect();
    store.append(session, records).await.unwrap();
    store.indexed().await;
    let mut query = request(session);
    query.limit = PromptIndexLimit::default();
    let started = Instant::now();
    let first = store.prompt_index(query.clone()).await.unwrap();
    let first_time = started.elapsed();
    let first_bytes = serde_json::to_vec(&first).unwrap().len();
    let mut page = first;
    let mut entries = page.entries().len();
    let mut pages = 1_usize;
    while let Some(after) = page.next() {
        query.after = Some(after);
        query.epoch = Some(page.epoch());
        page = store.prompt_index(query.clone()).await.unwrap();
        entries = entries.saturating_add(page.entries().len());
        pages = pages.saturating_add(1);
    }
    assert_eq!(u64::try_from(entries).unwrap(), TURNS);
    writeln!(std::io::stdout().lock(), "prompt metadata: {} records, {entries} prompts, {pages} pages, first={first_time:?}, all={:?}, first_reply_bytes={first_bytes}", TURNS.saturating_mul(2).saturating_add(1), started.elapsed()).unwrap();
    store.close().await;
}
