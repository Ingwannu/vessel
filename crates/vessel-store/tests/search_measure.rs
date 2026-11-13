//! What the search index costs, measured (protocol 12, ADR 0037; the
//! plan's "Track A2: progress"). Run:
//! `cargo test -p vessel-store --release --test search_measure --
//! --ignored --nocapture`.

use fixtures::{data_dir, open};
use support::*;
use vessel_proto::AgentEvent;
use vessel_store::{Store, StoreConfig};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "builders shared with the log's tests; this file uses some"
)]
mod fixtures;

#[cfg(test)]
#[path = "support/messages.rs"]
#[expect(dead_code, reason = "builders shared with the search's tests")]
mod support;

/// What the index costs, measured (the plan's "Track A2: progress"; run
/// `cargo test -p vessel-store --release --test search -- --ignored
/// --nocapture`). Two stores take the same 220,000 appends one at a time
/// (100 sessions of 100 turns: a prompt, 20 reply chunks, the turn's end):
/// in one the replies are `message_chunk` records, which the index reads;
/// in the other `thought_chunk` records of the same size, which it passes
/// over (the cursor's read and update still run). Then the idle steps'
/// time, the database's size, queries, and catching a whole store up.
/// One line of the measurement's report, on a locked standard output.
macro_rules! say {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        writeln!(std::io::stdout().lock(), $($arg)*).unwrap();
    }};
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "a measurement, not a check: minutes in a debug build"]
async fn measure() {
    let (indexed_dir_temp, indexed_dir) = data_dir();
    let (plain_dir_temp, plain_dir) = data_dir();
    let indexed = open(&indexed_dir, StoreConfig::default()).await;
    let plain = open(&plain_dir, StoreConfig::default()).await;
    let with = measure::fill(&indexed, true).await;
    let without = measure::fill(&plain, false).await;
    say!("appends with the index:    {with}");
    say!("appends without (thoughts): {without}");
    let started = std::time::Instant::now();
    indexed.indexed().await;
    say!("idle steps after the last append: {:?}", started.elapsed());
    for (query, session) in [
        ("zebrafish", false),
        ("parser", false),
        ("parser error", false),
        ("세션", false),
        ("세션 검색", false),
        ("zq", false),
        ("parser", true),
    ] {
        say!("{}", measure::query(&indexed, query, session).await);
    }
    indexed.close().await;
    plain.close().await;
    say!(
        "database: {} with the index, {} without",
        measure::size(&indexed_dir),
        measure::size(&plain_dir)
    );
    drop((indexed, plain));

    let connection = rusqlite::Connection::open(indexed_dir.database_path()).unwrap();
    connection
        .execute_batch(
            "INSERT INTO search_index (search_index) VALUES ('delete-all');
             DELETE FROM search_messages;
             UPDATE sessions SET search_seq = 0, search_open = NULL;",
        )
        .unwrap();
    drop(connection);
    let started = std::time::Instant::now();
    let again = open(&indexed_dir, StoreConfig::default()).await;
    // Appends to another session meanwhile, one after another: how long
    // one waits on the writer while it catches up.
    let writer = again.clone();
    let meanwhile = tokio::spawn(async move { measure::meanwhile(&writer).await });
    again.indexed().await;
    say!(
        "catching up 220,000 records from nothing: {:?}",
        started.elapsed()
    );
    meanwhile.abort();
    drop((indexed_dir_temp, plain_dir_temp));
}

#[cfg(test)]
mod measure {
    use std::time::{Duration, Instant};

    use super::*;

    const SESSIONS: i64 = 100;
    const TURNS: u64 = 100;
    const CHUNKS: u64 = 20;
    const WORDS: &[&str] = &[
        "the",
        "parser",
        "frame",
        "session",
        "error",
        "node",
        "agent",
        "build",
        "test",
        "queue",
        "세션",
        "검색",
        "오류",
        "에이전트",
        "빌드",
        "로그",
        "테스트",
        "확인",
        "수정",
        "파일",
    ];

    /// A deterministic word stream.
    struct Words(u64);

    impl Words {
        fn next(&mut self, count: usize) -> String {
            let mut text = Vec::with_capacity(count);
            for _ in 0..count {
                self.0 = self
                    .0
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let pick = usize::try_from(self.0.checked_shr(33).unwrap()).unwrap();
                // One message in about a thousand holds the rare word.
                if pick.checked_rem(20_000) == Some(0) {
                    text.push("zebrafish");
                } else {
                    text.push(WORDS[pick.checked_rem(WORDS.len()).unwrap()]);
                }
            }
            text.join(" ")
        }
    }

    pub(super) struct Latency(Vec<Duration>);

    impl std::fmt::Display for Latency {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut sorted = self.0.clone();
            sorted.sort();
            // The `per_mille`th of the sorted waits.
            let at = |per_mille: usize| {
                let index = sorted
                    .len()
                    .saturating_sub(1)
                    .saturating_mul(per_mille)
                    .checked_div(1_000)
                    .unwrap();
                sorted[index]
            };
            let total: Duration = sorted.iter().sum();
            write!(
                f,
                "{} appends in {:?}: median {:?}, p99 {:?}, p99.9 {:?}, max {:?}",
                sorted.len(),
                total,
                at(500),
                at(990),
                at(999),
                sorted.last().unwrap()
            )
        }
    }

    pub(super) async fn fill(store: &Store, replies: bool) -> Latency {
        let mut words = Words(7);
        let mut taken = Vec::new();
        for created in 0..SESSIONS {
            let session = started(store, created).await;
            let mut seq = 1;
            for _ in 0..TURNS {
                let mut records = vec![prompt(seq, &words.next(16))];
                seq = seq.checked_add(1).unwrap();
                for _ in 0..CHUNKS {
                    let content = text(&format!("{} ", words.next(7)));
                    let event = if replies {
                        AgentEvent::MessageChunk { content }
                    } else {
                        AgentEvent::ThoughtChunk { content }
                    };
                    records.push(agent(seq, event));
                    seq = seq.checked_add(1).unwrap();
                }
                records.push(turn_ended(seq));
                seq = seq.checked_add(1).unwrap();
                for record in records {
                    let started = Instant::now();
                    store.append(session, vec![record]).await.unwrap();
                    taken.push(started.elapsed());
                }
            }
        }
        Latency(taken)
    }

    /// Append to a new session until aborted, printing each 200 appends'
    /// waits.
    pub(super) async fn meanwhile(store: &Store) {
        let session = started(store, SESSIONS).await;
        let mut taken = Vec::new();
        let mut seq = 1;
        loop {
            let started = Instant::now();
            store
                .append(session, vec![chunk(seq, "meanwhile ")])
                .await
                .unwrap();
            taken.push(started.elapsed());
            seq = seq.checked_add(1).unwrap();
            if taken.len() == 200 {
                say!("  meanwhile: {}", Latency(std::mem::take(&mut taken)));
            }
        }
    }

    pub(super) async fn query(store: &Store, query: &str, one: bool) -> String {
        let session = if one {
            store.sessions().await.unwrap().first().map(|info| info.id)
        } else {
            None
        };
        let mut times = Vec::new();
        let mut hits = 0;
        for _ in 0..20 {
            let mut request = request(query);
            request.session = session;
            let started = Instant::now();
            let page = store.search(request).await.unwrap();
            times.push(started.elapsed());
            hits = page.hits.len();
        }
        times.sort();
        format!(
            "query {query:?}{}: {hits} hits, median {:?}, max {:?}",
            if one { " in one session" } else { "" },
            times[times.len().checked_div(2).unwrap()],
            times.last().unwrap()
        )
    }

    pub(super) fn size(dir: &vessel_store::DataDir) -> String {
        let bytes = std::fs::metadata(dir.database_path()).unwrap().len();
        format!("{} KiB", bytes.checked_div(1_024).unwrap())
    }
}
