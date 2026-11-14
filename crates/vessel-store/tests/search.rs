//! The search index (protocol 12, ADR 0037): what is a message, when it is
//! found, paging, and that it follows the log through deletion, retention,
//! a restart and an upgrade.

use fixtures::{at, data_dir, open, status};
use support::*;
use vessel_proto::search::{MessageRole, SearchLimit};
use vessel_proto::{EventSeq, SessionStatus};
use vessel_store::{ByteCount, Retention, StoreConfig};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "builders shared with the log's tests; this file uses some"
)]
mod fixtures;

#[cfg(test)]
#[path = "support/messages.rs"]
mod support;
#[tokio::test]
async fn a_reply_is_found_once_it_ends_and_a_prompt_at_once() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = started(&store, 0).await;
    store
        .append(session, vec![prompt(1, "find the parser bug")])
        .await
        .unwrap();
    // A word split over two chunks, in two appends.
    store
        .append(session, vec![chunk(2, "The pars")])
        .await
        .unwrap();
    store
        .append(session, vec![chunk(3, "er is fine")])
        .await
        .unwrap();
    assert_eq!(
        shown(&search(&store, "parser").await),
        ["find the [parser] bug"]
    );

    store.append(session, vec![turn_ended(4)]).await.unwrap();
    let page = search(&store, "PARSER").await;
    assert_eq!(
        shown(&page),
        ["The [parser] is fine", "find the [parser] bug"]
    );
    let reply = page.hits.first().unwrap();
    assert_eq!(
        (reply.session, reply.seq, reply.role),
        (session, EventSeq::new(2), MessageRole::Reply)
    );
    assert_eq!(reply.at, at(2));
    assert_eq!(page.next, None);
    // Thoughts, tool calls and the like are not messages.
    assert!(search(&store, "end_turn").await.hits.is_empty());
}

#[tokio::test]
async fn short_words_and_hangul_are_found_inside_words() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = started(&store, 0).await;
    store
        .append(
            session,
            vec![
                prompt(1, "로그인세션을 검색합니다"),
                prompt(2, "parse_frame failed"),
            ],
        )
        .await
        .unwrap();
    // Two characters: no trigram, the plain walk.
    assert_eq!(
        shown(&search(&store, "세션").await),
        ["로그인[세션]을 검색합니다"]
    );
    // A long word through the index, a short one checked on it.
    assert_eq!(
        shown(&search(&store, "검색합 세션").await),
        ["로그인[세션]을 [검색합]니다"]
    );
    assert_eq!(
        shown(&search(&store, "frame").await),
        ["parse_[frame] failed"]
    );
    assert!(search(&store, "세션 없음").await.hits.is_empty());
    assert!(search(&store, "\"OR\" NEAR(").await.hits.is_empty());
}

#[tokio::test]
async fn pages_walk_newest_first_without_gaps_or_repeats() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let first = started(&store, 0).await;
    let second = started(&store, 1).await;
    for seq in 1..=13 {
        for session in [first, second] {
            store
                .append(session, vec![prompt(seq, &format!("needle {seq}"))])
                .await
                .unwrap();
        }
    }
    store.indexed().await;
    let mut seen = Vec::new();
    let mut paged = request("needle");
    paged.limit = SearchLimit::new(10);
    loop {
        let page = store.search(paged.clone()).await.unwrap();
        assert!(page.hits.len() <= 10);
        seen.extend(page.hits.iter().map(|hit| (hit.at, hit.session, hit.seq)));
        match page.next {
            Some(next) => paged.before = Some(next),
            None => break,
        }
    }
    assert_eq!(seen.len(), 26);
    let mut sorted = seen.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    sorted.dedup();
    assert_eq!(sorted.len(), 26, "no hit twice");
    assert!(
        seen.windows(2).all(|pair| pair[0].0 >= pair[1].0),
        "newest first"
    );

    let mut one = request("needle");
    one.session = Some(second);
    let page = store.search(one).await.unwrap();
    assert_eq!(page.hits.len(), 13);
    assert!(page.hits.iter().all(|hit| hit.session == second));
}

#[tokio::test]
async fn a_long_message_is_indexed_in_pieces_that_say_where_they_start() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = started(&store, 0).await;
    let long = format!("{} tail-word", "word ".repeat(4_000));
    store
        .append(
            session,
            vec![chunk(1, "head-word "), chunk(2, &long), turn_ended(3)],
        )
        .await
        .unwrap();
    let head = search(&store, "head-word").await;
    let tail = search(&store, "tail-word").await;
    assert_eq!(head.hits.len(), 1);
    assert_eq!(tail.hits.len(), 1);
    // Every piece of one run points at the run's first record.
    assert_eq!(head.hits.first().unwrap().seq, EventSeq::new(1));
    assert_eq!(tail.hits.first().unwrap().seq, EventSeq::new(1));
    assert!(search(&store, "word").await.hits.len() >= 3);
}

#[tokio::test]
async fn deleting_a_session_or_pruning_its_records_takes_their_messages() {
    let (_temp, dir) = data_dir();
    let config = StoreConfig {
        retention: Retention {
            max_session_bytes: ByteCount::new(2_000),
            ..Retention::default()
        },
        ..StoreConfig::default()
    };
    let store = open(&dir, config).await;
    let kept = started(&store, 0).await;
    store
        .append(kept, vec![prompt(1, "oldest needle")])
        .await
        .unwrap();
    for seq in 2..40 {
        store
            .append(kept, vec![prompt(seq, &format!("newer needle {seq}"))])
            .await
            .unwrap();
    }
    assert!(
        search(&store, "oldest").await.hits.is_empty(),
        "pruned with its record"
    );
    assert!(!search(&store, "newer").await.hits.is_empty());

    let gone = started(&store, 1).await;
    store
        .append(
            gone,
            vec![
                prompt(1, "deleted words"),
                status(2, SessionStatus::Interrupted),
            ],
        )
        .await
        .unwrap();
    assert_eq!(search(&store, "deleted").await.hits.len(), 1);
    store.delete_session(gone).await.unwrap();
    assert!(search(&store, "deleted").await.hits.is_empty());
    store.close().await;
    drop(store);
    // The removed session's rows were deleted by the idle steps, and the
    // index and its table agree.
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    let left: i64 = connection
        .query_row(
            "SELECT count(*) FROM search_messages WHERE session_id = ?1",
            [gone.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(left, 0);
    let gone_marks: i64 = connection
        .query_row("SELECT count(*) FROM search_gone", [], |row| row.get(0))
        .unwrap();
    assert_eq!(gone_marks, 0);
    let _rows = connection
        .execute(
            "INSERT INTO search_index (search_index, rank) VALUES ('integrity-check', 1)",
            [],
        )
        .unwrap();
}

#[tokio::test]
async fn a_reply_cut_by_a_restart_is_indexed_whole() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = started(&store, 0).await;
    store
        .append(
            session,
            vec![chunk(1, "a reply cut "), chunk(2, "by a restart")],
        )
        .await
        .unwrap();
    store.close().await;
    drop(store);

    // Opening interrupts the session (a final status the writer did not
    // see); the idle catch-up ends the run from the log.
    let store = open(&dir, StoreConfig::default()).await;
    store.indexed().await;
    assert_eq!(
        shown(&search(&store, "reply restart").await),
        ["a [reply] cut by a [restart]"]
    );
}

#[tokio::test]
async fn sessions_from_before_the_index_are_caught_up() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let mut sessions = Vec::new();
    for created in 0..3 {
        let session = started(&store, created).await;
        let records: Vec<_> = (1..3_000)
            .map(|seq| prompt(seq, &format!("old words {created}-{seq}")))
            .collect();
        store.append(session, records).await.unwrap();
        sessions.push(session);
    }
    store.close().await;
    drop(store);
    // As a database from before schema 13 is after its migration.
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    connection
        .execute_batch(
            "DELETE FROM search_messages;
             UPDATE sessions SET search_seq = 0, search_open = NULL;",
        )
        .unwrap();
    drop(connection);

    let store = open(&dir, StoreConfig::default()).await;
    store.indexed().await;
    let mut one = request("words 2-2999");
    one.limit = SearchLimit::new(100);
    let page = store.search(one).await.unwrap();
    assert_eq!(page.hits.len(), 1);
    let mut every = request("old");
    every.limit = SearchLimit::new(100);
    let mut found = 0;
    loop {
        let page = store.search(every.clone()).await.unwrap();
        found += page.hits.len();
        match page.next {
            Some(next) => every.before = Some(next),
            None => break,
        }
    }
    assert_eq!(found, 3 * 2_999);
}
