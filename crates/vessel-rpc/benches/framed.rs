//! Benchmark of `Framed` (`docs/PERFORMANCE.md`): protocol frames over a
//! byte stream, the transport of the node's local socket and of SSH stdio
//! (`vesseld attach`).
//!
//! Run: `cargo bench -p vessel-rpc --bench framed`.
//!
//! One end writes a burst of frames, the other reads them back: the
//! length prefix, the copy into and out of the pipe, and the UTF-8 check
//! on every frame read. The pipe is tokio's in-memory duplex with a 64 KiB
//! buffer (a socket's order of size), so no system call is measured; the
//! frame's JSON text itself is `vessel-proto`'s `frames` benchmark.

#![expect(
    clippy::expect_used,
    reason = "a benchmark whose operation fails has nothing to measure: it stops and says why"
)]

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::runtime::Runtime;
use vessel_rpc::{Framed, MAX_FRAME};

/// Frames in one measured burst.
const BURST: usize = 256;

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    framed(&mut criterion);
    criterion.final_summary();
}

fn framed(criterion: &mut Criterion) {
    let runtime = Runtime::new().expect("a tokio runtime");
    let mut group = criterion.benchmark_group("rpc/framed");
    // A terminal event's frame (4 KiB of output, base64) and a message
    // chunk's, by their lengths in `vessel-proto`'s benchmark.
    for (name, length) in [("terminal-4KiB", 5_600_usize), ("message-chunk", 220)] {
        let frame = "x".repeat(length);
        let bytes = u64::try_from(length.saturating_mul(BURST)).expect("a burst's size fits");
        let _group = group.throughput(Throughput::Bytes(bytes));
        let _bench = group.bench_function(BenchmarkId::new("write-read", name), |bench| {
            bench.iter(|| {
                runtime.block_on(async {
                    let (near, far) = tokio::io::duplex(64 * 1024);
                    let (near_read, near_write) = tokio::io::split(near);
                    let (far_read, far_write) = tokio::io::split(far);
                    let mut writer = Framed::new(near_read, near_write, MAX_FRAME);
                    let mut reader = Framed::new(far_read, far_write, MAX_FRAME);
                    let frames = frame.clone();
                    let written = tokio::spawn(async move {
                        for _frame in 0..BURST {
                            writer
                                .feed(frames.clone())
                                .await
                                .expect("the frame is written");
                        }
                        writer.flush().await.expect("the burst is flushed");
                        writer
                    });
                    for _frame in 0..BURST {
                        let read = reader
                            .next()
                            .await
                            .expect("a frame arrives")
                            .expect("the frame is whole");
                        drop(black_box(read));
                    }
                    drop(written.await.expect("the writer finishes"));
                });
            });
        });
    }
    group.finish();
}
