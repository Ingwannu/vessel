//! Durable snapshot metadata remains paged across appends and rejects rewinds.

#![cfg(test)]

use std::time::Duration;
use vessel_proto::prompt_index::{ListSessionPrompts, PromptIndexLimit};
use vessel_proto::{EventSeq, SessionEvent, SessionId};
use vessel_store::{StoreConfig, StoreError};

// Safety only: frames/channels establish ordering; expiry always fails.
const SAFETY: Duration = Duration::from_secs(30);
const PHASE: Duration = Duration::from_secs(5);

async fn phase<T>(name: &str, wait: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, wait)
        .await
        .unwrap_or_else(|error| panic!("prompt store {name} stalled: {error}"))
}

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

async fn rewound_snapshot(
    store: &vessel_store::Store,
    session: SessionId,
) -> vessel_proto::prompt_index::PromptIndexPage {
    phase(
        "fixture append",
        store.append(
            session,
            vec![messages::prompt(1, "first"), messages::prompt(2, "second")],
        ),
    )
    .await
    .unwrap();
    let first = phase("metadata snapshot", store.prompt_index(request(session)))
        .await
        .unwrap();
    assert_eq!(first.entries().first().unwrap().preview.as_str(), "first");
    assert_eq!(first.next(), Some(EventSeq::new(1)));
    phase(
        "fixture append",
        store.append(session, vec![messages::prompt(3, "third")]),
    )
    .await
    .unwrap();
    let mut next = request(session);
    next.after = first.next();
    next.epoch = Some(first.epoch());
    let second = phase(
        "continued metadata snapshot",
        store.prompt_index(next.clone()),
    )
    .await
    .unwrap();
    assert_eq!(second.entries().first().unwrap().preview.as_str(), "second");
    assert_eq!(first.epoch(), second.epoch());
    phase(
        "fixture append",
        store.append(
            session,
            vec![fixtures::record(
                4,
                SessionEvent::Rewound {
                    after: EventSeq::new(1),
                    harness_session: vessel_proto::HarnessSessionId::new("fake-history"),
                    files_changed: 0,
                },
            )],
        ),
    )
    .await
    .unwrap();
    assert!(matches!(
        phase("rewound metadata refusal", store.prompt_index(next)).await,
        Err(StoreError::PromptIndexChanged)
    ));
    let visible = phase("metadata snapshot", store.prompt_index(request(session)))
        .await
        .unwrap();
    assert_eq!(visible.entries().len(), 1);
    assert_eq!(visible.next(), None);
    visible
}

#[tokio::test]
async fn prompt_index_reads_one_snapshot_and_survives_reopen_without_a_new_schema() {
    vessel_testkit::within(SAFETY, async {
        let (_temp, dir) = fixtures::data_dir();
        let store = phase(
            "open or reopen",
            fixtures::open(&dir, StoreConfig::default()),
        )
        .await;
        let session = phase("create fixture session", messages::started(&store, 0)).await;
        assert!(
            phase(
                "empty metadata snapshot",
                store.prompt_index(request(session))
            )
            .await
            .unwrap()
            .entries()
            .is_empty()
        );
        let visible = rewound_snapshot(&store, session).await;
        // Reopen after the writer/index workers release their handles; dropping a
        // facade alone does not synchronize their asynchronous shutdown.
        phase("close", store.close()).await;
        let reopened = phase(
            "open or reopen",
            fixtures::open(&dir, StoreConfig::default()),
        )
        .await;
        assert_eq!(
            phase(
                "durable reopened metadata",
                reopened.prompt_index(request(session))
            )
            .await
            .unwrap(),
            visible
        );
        assert!(matches!(
            phase(
                "missing session refusal",
                reopened.prompt_index(request(SessionId::random()))
            )
            .await,
            Err(StoreError::UnknownSession(_))
        ));
        phase("reopened close", reopened.close()).await;
    })
    .await
    .expect("prompt store safety deadline");
}

#[tokio::test]
async fn prompt_index_retention_invalidates_a_continuation_and_never_returns_pruned_prompts() {
    vessel_testkit::within(SAFETY, async {
        let (_temp, dir) = fixtures::data_dir();
        let config = StoreConfig {
            retention: vessel_store::Retention {
                max_session_bytes: vessel_store::ByteCount::new(512),
                ..Default::default()
            },
            ..Default::default()
        };
        let store = phase("retention open", fixtures::open(&dir, config)).await;
        let session = phase("create fixture session", messages::started(&store, 0)).await;
        phase(
            "fixture append",
            store.append(
                session,
                vec![messages::prompt(1, "first"), messages::prompt(2, "second")],
            ),
        )
        .await
        .unwrap();
        let first = phase("metadata snapshot", store.prompt_index(request(session)))
            .await
            .unwrap();
        assert_eq!(first.next(), Some(EventSeq::new(1)));
        phase(
            "fixture append",
            store.append(session, vec![messages::prompt(3, &"x".repeat(4_096))]),
        )
        .await
        .unwrap();
        let mut continuation = request(session);
        continuation.after = first.next();
        continuation.epoch = Some(first.epoch());
        assert!(matches!(
            phase(
                "pruned continuation refusal",
                store.prompt_index(continuation)
            )
            .await,
            Err(StoreError::PromptIndexChanged)
        ));
        let retained = phase("metadata snapshot", store.prompt_index(request(session)))
            .await
            .unwrap();
        assert!(retained.epoch().retained_from() > first.epoch().retained_from());
        assert_eq!(retained.entries().len(), 1);
        assert_eq!(retained.entries()[0].seq, EventSeq::new(3));
        phase("close", store.close()).await;
    })
    .await
    .expect("prompt store safety deadline");
}

/// Report timings without a performance threshold; safety deadlines identify stalled fixtures.
#[tokio::test]
#[ignore = "measurement of repeated snapshot paging; run explicitly with --ignored --nocapture"]
async fn prompt_index_measure_metadata_pages_over_twenty_thousand_records() {
    vessel_testkit::within(SAFETY, async {
        use std::io::Write as _;
        use std::time::Instant;
        const TURNS: u64 = 10_000;
        const CHUNK_BYTES: usize = 1_024;
        let (_temp, dir) = fixtures::data_dir();
        let store = phase("open or reopen", fixtures::open(&dir, StoreConfig::default())).await;
        let session = phase("create fixture session", messages::started(&store, 0)).await;
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
        phase("measurement append", store.append(session, records)).await.unwrap();
        phase("measurement indexed", store.indexed()).await;
        let mut query = request(session);
        query.limit = PromptIndexLimit::default();
        let started = Instant::now();
        let first = phase("measurement page", store.prompt_index(query.clone())).await.unwrap();
        let first_time = started.elapsed();
        let first_bytes = serde_json::to_vec(&first).unwrap().len();
        let mut page = first;
        let mut entries = page.entries().len();
        let mut pages = 1_usize;
        while let Some(after) = page.next() {
            query.after = Some(after);
            query.epoch = Some(page.epoch());
            page = phase("measurement page", store.prompt_index(query.clone())).await.unwrap();
            entries = entries.saturating_add(page.entries().len());
            pages = pages.saturating_add(1);
        }
        assert_eq!(u64::try_from(entries).unwrap(), TURNS);
        writeln!(std::io::stdout().lock(), "prompt metadata: {} records, {entries} prompts, {pages} pages, first={first_time:?}, all={:?}, first_reply_bytes={first_bytes}", TURNS.saturating_mul(2).saturating_add(1), started.elapsed()).unwrap();
        phase("close", store.close()).await;
    }).await.expect("prompt store safety deadline");
}
