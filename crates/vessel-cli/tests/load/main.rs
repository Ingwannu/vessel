//! The load test (`docs/PERFORMANCE.md`): many sessions on one node
//! emitting through `vessel ui`'s bridge to one browser tab.
//!
//! Ignored by the gate; run on demand, in the profile being measured:
//!
//! ```text
//! cargo test --release -p vessel-cli --test load -- --ignored --nocapture
//! ```
//!
//! `VESSEL_LOAD_SESSIONS` sets the number of terminal sessions (default
//! 32, at most 64: one tab follows at most 64 sessions).
//!
//! [Decision Log]
//! - Purpose: a number for "is it fast" that covers the whole path a page
//!   sees: a program's output read by the node, numbered, persisted and
//!   broadcast, encoded for the node's connection, decoded and encoded
//!   again by the bridge, and read by the tab.
//! - Choice: the real binaries, each its own process (`vesseld`, `vessel
//!   ui`), so each one's resident memory can be read on its own; the tab
//!   is this test, speaking the page's frames. Latency is from a record's
//!   `at` (when the session numbered it, on the node) to when the tab has
//!   decoded it: the same machine's clock on both ends.
//! - Workloads: a burst (every terminal session writes 500 KB at once:
//!   the throughput ceiling, where latency is queueing); a paced stream
//!   (each session 20 lines every 100 ms, far below the ceiling: the
//!   latency a person watching sees); and structured sessions flooding
//!   message chunks (the agent path, events instead of bytes).
//! - Not in the gate: it takes about a minute, and its figures depend on
//!   the machine. The benchmarks under each crate's `benches/` measure
//!   the pieces.

// `cfg(test)`, as in the CLI's other test crates: the fixtures' helpers
// are test code, under the test lints.
#[cfg(test)]
mod fleet;
#[cfg(test)]
mod report;
#[cfg(test)]
mod run;
#[cfg(test)]
mod tab;

use std::fmt::Write as _;
use std::io::Write as _;

use fleet::Fleet;
use report::{line, rss};
use run::Run;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "a measurement, run on demand: docs/PERFORMANCE.md"]
async fn many_sessions_through_the_bridge() {
    let sessions = std::env::var("VESSEL_LOAD_SESSIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(32)
        .clamp(1, 64);
    let fleet = Fleet::start().await;
    let (node, ui) = fleet.pids();
    let mut report = String::new();
    let _written = writeln!(
        report,
        "\n{sessions} terminal sessions and 8 structured sessions on one node, one tab"
    );
    let mut run = Run::open(&fleet).await;
    rss(&mut report, "started, no sessions", node, ui);
    run.terminals(sessions).await;
    rss(&mut report, "terminal sessions open, idle", node, ui);
    let burst = run.burst(5_000, 100).await;
    line(&mut report, "terminal burst", &burst);
    rss(&mut report, "after the burst", node, ui);
    let paced = run.paced(20, 100, 10).await;
    line(&mut report, "terminal paced", &paced);
    run.structured(8).await;
    let agents = run.agent_flood(5).await;
    line(&mut report, "agent flood", &agents);
    rss(&mut report, "after the agent flood", node, ui);
    drop(run);
    let mut fleet = fleet;
    stop(&mut fleet).await;
    let _shown = std::io::stdout().lock().write_all(report.as_bytes());
}

#[cfg(test)]
/// Stop the node the way its service manager would (`SIGTERM`), so it
/// kills its sessions' process groups; then the UI.
async fn stop(fleet: &mut Fleet) {
    #[cfg(unix)]
    {
        let (node, _ui) = fleet.pids();
        let status = tokio::process::Command::new("kill")
            .args(["-TERM", &node.to_string()])
            .status()
            .await
            .unwrap();
        assert!(status.success());
        let _exited = fleet.node.wait().await.unwrap();
    }
    fleet.ui.kill().await.unwrap();
}
