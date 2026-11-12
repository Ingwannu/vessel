//! Benchmark of what the bridge does with each record of a followed
//! session (`docs/PERFORMANCE.md`): the node's frame text decoded into the
//! protocol's `ServerFrame` (`vessel-client`'s connection does that), and
//! the record written again as the browser's `wire::ServerFrame::Event`
//! (`bridge::write`).
//!
//! Run: `cargo bench -p vessel-web --bench bridge`.
//!
//! The record moves by value from one to the other, as in the bridge, so
//! the figure is the bridge's own work per record, without the sockets
//! (the load test, `vessel-cli`'s `load`, measures those end to end).

#![expect(
    clippy::expect_used,
    reason = "a benchmark whose operation fails has nothing to measure: it stops and says why"
)]

use std::hint::black_box;

use bytes::Bytes;
use chrono::Utc;
use criterion::{BenchmarkId, Criterion, Throughput};
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, SessionEvent, SubscriptionId, TerminalBytes,
    TerminalOutput,
};
use vessel_web::wire::{ServerFrame, StreamId};

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    forward(&mut criterion);
    criterion.final_summary();
}

/// The node's text for a subscription event carrying `event`.
fn node_frame(event: SessionEvent) -> String {
    serde_json::to_string(&vessel_proto::ServerFrame::Event {
        subscription: SubscriptionId::new(3),
        record: EventRecord {
            seq: EventSeq::new(120_431),
            at: Utc::now(),
            event,
        },
    })
    .expect("the frame encodes")
}

fn forward(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("web/bridge-forward");
    let samples = [
        (
            "terminal-4KiB",
            node_frame(SessionEvent::Terminal(TerminalOutput {
                data: TerminalBytes::new(Bytes::from(b"0123456789abcdef".repeat(256))),
            })),
        ),
        (
            "message-chunk",
            node_frame(SessionEvent::Agent {
                event: AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: "The function returns early when the list is empty, ".to_owned(),
                    },
                },
            }),
        ),
    ];
    for (name, text) in samples {
        let bytes = u64::try_from(text.len()).expect("a frame's length fits");
        let _group = group.throughput(Throughput::Bytes(bytes));
        let _bench = group.bench_function(BenchmarkId::from_parameter(name), |bench| {
            bench.iter(|| {
                let frame = serde_json::from_str::<vessel_proto::ServerFrame>(black_box(&text))
                    .expect("the node's frame decodes");
                let vessel_proto::ServerFrame::Event { record, .. } = frame else {
                    return String::new();
                };
                serde_json::to_string(&ServerFrame::Event {
                    stream: StreamId::new(7),
                    record,
                })
                .expect("the browser's frame encodes")
            });
        });
    }
    group.finish();
}
