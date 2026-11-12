//! `port.list`: the TCP listeners of the node's sessions' and terminals'
//! process trees (protocol 11, ADR 0031), found by `vessel_proc::listening`.
//!
//! [Decision Log]
//! - Purpose: a person (or an agent, its own) wants the port a dev server
//!   opened; the node finds it without a privileged call.
//! - Bounded in how often: one task scans, at most every [`PERIOD`], and
//!   only while someone asked within [`INTEREST`]; a node nobody asks
//!   never scans. A scan runs one at a time (on the blocking pool on
//!   Linux, as children with a time limit elsewhere), within
//!   `ScanLimits::DEFAULT`; at most [`MAX_PORTS`] are reported.
//! - The list is a `watch`: a `port.list` with `changed_from` waits for the
//!   next revision, and the revision moves only when the list or the
//!   scan's state does, so a quiet machine wakes no waiter.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{Notify, watch};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vessel_proc::listening::{self, ScanLimits};
use vessel_proto::ports::{ListeningPort, PortList, PortOwner, PortScan, ProcessName};

/// At most one scan this often (GOAT's period).
pub(crate) const PERIOD: Duration = Duration::from_secs(3);
/// Scans go on this long after the last `port.list`.
pub(crate) const INTEREST: Duration = Duration::from_secs(60);
/// The most ports reported.
pub(crate) const MAX_PORTS: usize = 256;

/// The roots of the trees to scan, each with its owner.
pub(crate) type Roots = Vec<(u32, PortOwner)>;

/// The node's ports, as last found.
#[derive(Debug)]
pub(crate) struct Ports {
    found: watch::Sender<PortList>,
    asked: watch::Sender<Option<Instant>>,
    wake: Notify,
}

impl Ports {
    /// An empty list, and the scanner task, which asks `roots` for the
    /// trees at every scan and stops with `stop`.
    pub(crate) fn start(
        roots: impl Fn() -> Roots + Send + Sync + 'static,
        stop: CancellationToken,
        tasks: &tokio_util::task::TaskTracker,
    ) -> Arc<Self> {
        let ports = Arc::new(Self {
            found: watch::Sender::new(PortList {
                ports: Vec::new(),
                scan: PortScan::Pending,
                revision: 0,
                more: false,
            }),
            asked: watch::Sender::new(None),
            wake: Notify::new(),
        });
        let scanner = Arc::clone(&ports);
        drop(tasks.spawn(async move {
            tokio::select! {
                () = stop.cancelled() => {}
                () = scanner.scan_while_asked(roots) => {}
            }
        }));
        ports
    }

    /// The list now, or once its revision differs from `changed_from`
    /// (when `wait`); every call counts as interest, so scans run.
    pub(crate) async fn list(&self, changed_from: Option<u64>, wait: bool) -> PortList {
        let _previous = self.asked.send_replace(Some(Instant::now()));
        self.wake.notify_one();
        let mut found = self.found.subscribe();
        if let (Some(from), true) = (changed_from, wait) {
            let changed = found.wait_for(|list| list.revision != from);
            let _timed_out = tokio::time::timeout(
                Duration::from_secs(u64::from(super::MAX_WAIT_SECS)),
                changed,
            )
            .await;
        }
        found.borrow().clone()
    }

    async fn scan_while_asked(&self, roots: impl Fn() -> Roots) {
        loop {
            let recent = self
                .asked
                .borrow()
                .is_some_and(|at| at.elapsed() < INTEREST);
            if !recent {
                self.wake.notified().await;
                continue;
            }
            let started = Instant::now();
            self.scan_once(&roots()).await;
            let rest = PERIOD.saturating_sub(started.elapsed());
            tokio::time::sleep(rest).await;
        }
    }

    async fn scan_once(&self, roots: &Roots) {
        let pids: Vec<u32> = roots.iter().map(|(pid, _)| *pid).collect();
        let owners: HashMap<u32, PortOwner> = roots.iter().copied().collect();
        let (mut ports, scan) = match listening::scan(&pids, ScanLimits::DEFAULT).await {
            Ok(scan) => {
                if scan.truncated {
                    tracing::debug!("a port scan stopped at its bounds");
                }
                let ports: Vec<ListeningPort> = scan
                    .listeners
                    .into_iter()
                    .filter_map(|listener| {
                        Some(ListeningPort {
                            port: listener.port,
                            address: listener.address,
                            pid: listener.pid,
                            process: listener
                                .process
                                .as_deref()
                                .and_then(ProcessName::from_system),
                            owner: *owners.get(&listener.root)?,
                        })
                    })
                    .collect();
                (ports, PortScan::Scanned { at: Utc::now() })
            }
            Err(error) => {
                tracing::debug!(%error, "a port scan failed");
                (
                    Vec::new(),
                    PortScan::Unavailable {
                        reason: error.to_string(),
                    },
                )
            }
        };
        ports.sort_by(|a, b| {
            a.owner
                .cmp(&b.owner)
                .then(a.port.cmp(&b.port))
                .then_with(|| a.address.cmp(&b.address))
        });
        let more = ports.len() > MAX_PORTS;
        ports.truncate(MAX_PORTS);
        let _woken = self.found.send_if_modified(|list| {
            // The scan's time moves at every scan; a waiter is woken only
            // when what it would show changed.
            let same_state = matches!(
                (&list.scan, &scan),
                (PortScan::Scanned { .. }, PortScan::Scanned { .. })
            ) || list.scan == scan;
            let changed = list.ports != ports || list.more != more || !same_state;
            list.scan = scan;
            if changed {
                list.ports = ports;
                list.more = more;
                list.revision = list.revision.wrapping_add(1);
            }
            changed
        });
    }
}
