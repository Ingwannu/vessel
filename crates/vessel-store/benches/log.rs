//! Benchmarks of the event log (`docs/PERFORMANCE.md`): appending a
//! session's records, as the node's persistence does, and reading a page
//! of them back, as `session.read` and a subscriber's gap fill do.
//!
//! Run: `cargo bench -p vessel-store --bench log`.
//!
//! The records are the two shapes that dominate a log: terminal output as
//! the PTY reader hands it over (up to 16 KiB a read; 4 KiB is the size
//! seen most under a steady stream, `docs/PERFORMANCE.md`), and an agent's
//! message chunk (a few dozen bytes). Appends go through the public
//! `Store`, so each one includes the queue to the writer thread, the
//! transaction and the commit, which is what a caller waits for.

#![expect(
    clippy::expect_used,
    reason = "a benchmark that cannot set up, or whose operation fails, has nothing to \
              measure: it stops and says why"
)]

use std::hint::black_box;
use std::sync::Arc;

use bytes::Bytes;
use chrono::Utc;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};
use tokio::runtime::Runtime;
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, HarnessKind, SessionEvent, SessionId,
    SessionInfo, SessionMode, SessionSpec, TerminalBytes, TerminalOutput, TerminalSize,
};
use vessel_store::{DataDir, Store, StoreConfig};

/// Records a session's log holds before reads are measured.
const READ_LOG: u64 = 10_000;

/// Records a measured read asks for: under the store's page byte limit
/// (1 MiB) for the terminal records too, so every read returns them all.
const PAGE: u32 = 128;

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    appends(&mut criterion);
    reads(&mut criterion);
    criterion.final_summary();
}

/// One of the two record shapes.
#[derive(Clone, Copy)]
enum Shape {
    Terminal4K,
    MessageChunk,
}

impl Shape {
    const fn name(self) -> &'static str {
        match self {
            Self::Terminal4K => "terminal-4KiB",
            Self::MessageChunk => "message-chunk",
        }
    }

    fn event(self) -> SessionEvent {
        match self {
            Self::Terminal4K => SessionEvent::Terminal(TerminalOutput {
                data: TerminalBytes::new(Bytes::from(vec![b'x'; 4096])),
            }),
            Self::MessageChunk => SessionEvent::Agent {
                event: AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: "The function returns early when the list is empty, ".to_owned(),
                    },
                },
            },
        }
    }
}

/// A store in a temporary directory with one session in it.
struct Log {
    _dir: tempfile::TempDir,
    store: Store,
    session: SessionId,
    next: u64,
}

impl Log {
    fn open(runtime: &Runtime) -> Self {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let data = DataDir::new(dir.path());
        let store = runtime
            .block_on(Store::open(&data, StoreConfig::default()))
            .expect("the store opens")
            .store;
        let session = SessionId::random();
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        );
        runtime
            .block_on(store.create_session(SessionInfo::new(session, spec, Utc::now())))
            .expect("the session is stored");
        Self {
            _dir: dir,
            store,
            session,
            next: 0,
        }
    }

    /// The next `count` records of the log, numbered to continue it.
    fn batch(&mut self, shape: Shape, count: u64) -> Vec<Arc<EventRecord>> {
        let event = shape.event();
        let first = self.next;
        self.next = first.checked_add(count).expect("the sequence fits");
        (first..self.next)
            .map(|seq| {
                Arc::new(EventRecord {
                    seq: EventSeq::new(seq),
                    at: Utc::now(),
                    event: event.clone(),
                })
            })
            .collect()
    }
}

/// `Store::append` of a batch: 1 record (a quiet session), 16, and 256
/// (the node's largest batch, `vessel-node`'s `persist::BATCH`).
fn appends(criterion: &mut Criterion) {
    let runtime = Runtime::new().expect("a tokio runtime");
    let mut group = criterion.benchmark_group("store/append");
    for shape in [Shape::Terminal4K, Shape::MessageChunk] {
        let mut log = Log::open(&runtime);
        let (store, session) = (log.store.clone(), log.session);
        for count in [1_u64, 16, 256] {
            let _group = group.throughput(Throughput::Elements(count));
            let _bench = group.bench_function(BenchmarkId::new(shape.name(), count), |bench| {
                bench.iter_batched(
                    || log.batch(shape, count),
                    |records| {
                        runtime
                            .block_on(store.append(session, records))
                            .expect("the records are appended");
                    },
                    BatchSize::SmallInput,
                );
            });
        }
        runtime.block_on(store.close());
    }
    group.finish();
}

/// `Store::read_events`: a page of [`PAGE`] records from the middle of a
/// log of [`READ_LOG`].
fn reads(criterion: &mut Criterion) {
    let runtime = Runtime::new().expect("a tokio runtime");
    let mut group = criterion.benchmark_group("store/read");
    for shape in [Shape::Terminal4K, Shape::MessageChunk] {
        let mut log = Log::open(&runtime);
        let mut written = 0;
        while written < READ_LOG {
            let records = log.batch(shape, 256);
            written = log.next;
            runtime
                .block_on(log.store.append(log.session, records))
                .expect("the log is filled");
        }
        let from = EventSeq::new(READ_LOG.checked_div(2).unwrap_or_default());
        let page = runtime
            .block_on(log.store.read_events(log.session, from, PAGE))
            .expect("the page is read");
        assert_eq!(
            page.records.len(),
            usize::try_from(PAGE).expect("a page length fits"),
            "a measured read returns the whole page"
        );
        let _group = group.throughput(Throughput::Elements(u64::from(PAGE)));
        let _bench = group.bench_function(BenchmarkId::new(shape.name(), PAGE), |bench| {
            bench.iter(|| {
                let page = runtime
                    .block_on(log.store.read_events(log.session, from, PAGE))
                    .expect("the page is read");
                black_box(page)
            });
        });
        runtime.block_on(log.store.close());
    }
    group.finish();
}
