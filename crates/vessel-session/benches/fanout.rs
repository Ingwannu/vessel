//! Benchmarks of a session's output path (`docs/PERFORMANCE.md`): a
//! terminal program's output read from its PTY, numbered, handed to the
//! durable channel, kept in the ring, broadcast to every follower, and
//! the session's info (its place on the board included) updated, which
//! is `Publisher::commit` for every record; and the board's reducer on
//! its own.
//!
//! Run: `cargo bench -p vessel-session --bench fanout`.
//!
//! The program is `fake-tui` (`vessel-testkit`) under a real PTY: one
//! measured iteration asks it to `flood` [`LINES`] lines of [`WIDTH`]
//! bytes and ends when every follower has read its `flooded` line. A
//! drain task stands in for the node's persistence, as the session tests
//! do. The figure is bytes a second per follower set, so 1, 4 and 16
//! followers show what each extra reader costs.

#![expect(
    clippy::expect_used,
    reason = "a benchmark that cannot set up, or whose session fails, has nothing to measure: \
              it stops and says why"
)]

use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use chrono::Utc;
use criterion::{BenchmarkId, Criterion, Throughput};
use tokio::runtime::Runtime;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use vessel_harness::Lookup;
use vessel_harness::fakes::{Role, install};
use vessel_proto::{
    Activity, EventRecord, HarnessKind, SessionEvent, SessionInput, SessionMode, SessionSpec,
    TerminalBytes, TerminalInput, TerminalSize,
};
use vessel_session::board::{self, PullRequests, SessionFacts, WorkerFacts};
use vessel_session::{
    DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, RuntimeConfig,
    SessionHandle, SessionRuntime,
};
use vessel_testkit::Fake;

/// Lines one measured flood writes.
const LINES: u64 = 2_000;
/// Bytes in each line, its line break included.
const WIDTH: u64 = 100;

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    fanout(&mut criterion);
    derive(&mut criterion);
    criterion.final_summary();
}

/// A runtime whose search path holds only `fake-tui`, named `claude`.
struct Fixture {
    runtime: SessionRuntime,
    _shutdown: CancellationToken,
    bin: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let bin = std::env::temp_dir().join(format!("vessel-bench-fanout-{}", std::process::id()));
        std::fs::create_dir_all(&bin).expect("a directory for the fake");
        let tui = Fake::Tui.path().expect("fake-tui is built");
        drop(
            install(&bin, HarnessKind::ClaudeCode, Role::Terminal, &tui)
                .expect("fake-tui is installed"),
        );
        let shutdown = CancellationToken::new();
        let runtime = SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: Lookup::new([bin.clone()]),
                // Never the machine's own account.
                harness_env: vessel_harness::HarnessEnv::new(std::env::temp_dir()),
                ring_capacity: DEFAULT_RING_CAPACITY,
                durable_capacity: DEFAULT_DURABLE_CAPACITY,
                agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            },
            shutdown.clone(),
        );
        Self {
            runtime,
            _shutdown: shutdown,
            bin,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.bin));
    }
}

fn keys(text: &str) -> SessionInput {
    SessionInput::Terminal {
        input: TerminalInput::Data {
            data: TerminalBytes::new(Bytes::copy_from_slice(text.as_bytes())),
        },
    }
}

/// Why a follower stopped before its needle.
#[derive(Debug)]
enum Missed {
    /// It fell behind the broadcast by this many records: the run would
    /// measure the store's gap fill, not the fan-out.
    Lagged(#[expect(dead_code, reason = "shown by `Debug` when a run stops")] u64),
    /// The session ended first.
    Ended,
}

/// Read `handle`'s records from its next one until `needle` has been
/// printed; returns the records read. `ready` is told once the follower
/// is subscribed, so nothing the caller does next is missed.
async fn until(
    handle: &SessionHandle,
    needle: &'static [u8],
    ready: oneshot::Sender<()>,
) -> Result<u64, Missed> {
    let from = handle.info().next_seq;
    let mut follow = handle.follow(from).expect("the ring has the next record");
    // The caller may have stopped waiting; nothing to tell then.
    let _told = ready.send(());
    let mut tail: Vec<u8> = Vec::new();
    let mut records = 0_u64;
    for record in follow.cached.drain(..) {
        records = records.saturating_add(1);
        if seen(&mut tail, &record, needle) {
            return Ok(records);
        }
    }
    loop {
        let record = match follow.live.recv().await {
            Ok(record) => record,
            Err(RecvError::Lagged(missed)) => return Err(Missed::Lagged(missed)),
            Err(RecvError::Closed) => return Err(Missed::Ended),
        };
        records = records.saturating_add(1);
        if seen(&mut tail, &record, needle) {
            return Ok(records);
        }
    }
}

/// Whether `needle` is in the output so far (`tail` keeps the end of the
/// previous chunk, so a needle split across reads is found).
fn seen(tail: &mut Vec<u8>, record: &Arc<EventRecord>, needle: &[u8]) -> bool {
    let SessionEvent::Terminal(output) = &record.event else {
        return false;
    };
    tail.extend_from_slice(output.data.bytes());
    let found = tail.windows(needle.len()).any(|window| window == needle);
    let keep = tail.len().saturating_sub(needle.len());
    drop(tail.drain(..keep));
    found
}

fn fanout(criterion: &mut Criterion) {
    let tokio = Runtime::new().expect("a tokio runtime");
    let fixture = Fixture::new();
    let mut group = criterion.benchmark_group("session/terminal-fanout");
    let _group = group
        .sample_size(10)
        .measurement_time(Duration::from_secs(10))
        .throughput(Throughput::Bytes(LINES.saturating_mul(WIDTH)));
    for followers in [1_usize, 4, 16] {
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize {
                    cols: 200,
                    rows: 50,
                },
            },
        )
        .with_cwd(std::env::temp_dir());
        let created = tokio
            .block_on(fixture.runtime.create(spec))
            .expect("the session starts");
        let handle = created.handle;
        let mut durable = created.durable;
        let drain = tokio.spawn(async move { while durable.recv().await.is_some() {} });
        // Its first prompt: the program is up and reading.
        let (ready, started) = oneshot::channel();
        let first = handle.clone();
        let prompted = tokio.block_on(async move {
            let waiting = tokio::spawn(async move { until(&first, b"ready", ready).await });
            started.await.expect("the follower starts");
            waiting.await.expect("the follower finishes")
        });
        let _records = prompted.expect("the program starts");
        let command = format!("flood {LINES} {}\r", WIDTH.saturating_sub(2));
        let _bench = group.bench_function(BenchmarkId::from_parameter(followers), |bench| {
            bench.iter_custom(|iterations| {
                let mut spent = Duration::ZERO;
                for _flood in 0..iterations {
                    spent =
                        spent.saturating_add(tokio.block_on(flood(&handle, followers, &command)));
                }
                spent
            });
        });
        handle.kill();
        drop(tokio.block_on(drain));
    }
    group.finish();
    tokio.block_on(fixture.runtime.shutdown());
}

/// One flood read by `followers` followers; how long it took from the
/// command to the last follower's `flooded` line.
async fn flood(handle: &SessionHandle, followers: usize, command: &str) -> Duration {
    let mut readers = Vec::with_capacity(followers);
    for _follower in 0..followers {
        let (ready, started) = oneshot::channel();
        let handle = handle.clone();
        readers.push(tokio::spawn(async move {
            until(&handle, b"flooded ", ready).await
        }));
        started.await.expect("the follower starts");
    }
    let start = Instant::now();
    handle
        .send(keys(command))
        .await
        .expect("the command is sent");
    for reader in readers {
        let records = reader.await.expect("the follower finishes");
        let _records = black_box(records.expect("every follower reads the whole flood"));
    }
    start.elapsed()
}

/// `board::derive` for a session in the middle of a turn: the reducer
/// runs on every record a session commits.
fn derive(criterion: &mut Criterion) {
    let now = Utc::now();
    let facts = SessionFacts {
        worker: WorkerFacts {
            activity: Activity::Active,
            last_activity_at: now,
            has_signal: true,
            signal_expected: false,
            is_terminated: false,
        },
        interrupted: false,
        has_artifact_output: false,
        policy: board::Policy::default(),
    };
    let _bench = criterion.bench_function("session/board-derive", |bench| {
        bench.iter(|| {
            board::derive(
                black_box(&facts),
                PullRequests::Unseen,
                now,
                board::NO_SIGNAL_GRACE,
            )
        });
    });
}
