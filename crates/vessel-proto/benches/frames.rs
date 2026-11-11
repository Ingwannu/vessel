//! Benchmarks of a frame's text (`docs/PERFORMANCE.md`): encoding a
//! subscription event as the node writes it and decoding it as a client
//! (the CLI, the web bridge) reads it.
//!
//! Run: `cargo bench -p vessel-proto --bench frames`.
//!
//! `vessel-rpc` encodes a live event through a borrowed view of the same
//! shape (its `server::encode`, whose test pins that the bytes are equal),
//! so `ServerFrame::Event` here costs what the node pays per subscriber
//! and per record. The two records are the shapes that dominate a stream:
//! a 4 KiB PTY read (base64 on the wire) and an agent's message chunk.

#![expect(
    clippy::expect_used,
    reason = "a benchmark whose operation fails has nothing to measure: it stops and says why"
)]

use std::hint::black_box;

use bytes::Bytes;
use chrono::Utc;
use criterion::{BenchmarkId, Criterion, Throughput};
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, ServerFrame, SessionEvent, SubscriptionId,
    TerminalBytes, TerminalOutput,
};

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    frames(&mut criterion);
    criterion.final_summary();
}

/// The two record shapes, each in a subscription event.
fn samples() -> [(&'static str, ServerFrame); 2] {
    let event = |event| ServerFrame::Event {
        subscription: SubscriptionId::new(3),
        record: EventRecord {
            seq: EventSeq::new(120_431),
            at: Utc::now(),
            event,
        },
    };
    [
        (
            "terminal-4KiB",
            event(SessionEvent::Terminal(TerminalOutput {
                data: TerminalBytes::new(Bytes::from(b"0123456789abcdef".repeat(256))),
            })),
        ),
        (
            "message-chunk",
            event(SessionEvent::Agent {
                event: AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: "The function returns early when the list is empty, ".to_owned(),
                    },
                },
            }),
        ),
    ]
}

fn frames(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("proto/event-frame");
    for (name, frame) in samples() {
        let text = serde_json::to_string(&frame).expect("the frame encodes");
        let bytes = u64::try_from(text.len()).expect("a frame's length fits");
        let _group = group.throughput(Throughput::Bytes(bytes));
        let _bench = group.bench_function(BenchmarkId::new("encode", name), |bench| {
            bench.iter(|| serde_json::to_string(black_box(&frame)).expect("the frame encodes"));
        });
        let _bench = group.bench_function(BenchmarkId::new("decode", name), |bench| {
            bench.iter(|| {
                serde_json::from_str::<ServerFrame>(black_box(&text)).expect("the frame decodes")
            });
        });
    }
    group.finish();
}
