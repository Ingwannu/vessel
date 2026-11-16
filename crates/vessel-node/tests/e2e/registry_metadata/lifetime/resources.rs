//! Retain actual startup and shutdown owners when a validation observer goes away.
use crate::{
    secure::Relay,
    support::{Fixture, step},
};
use futures_util::future::join_all;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    sync::oneshot,
    task::{JoinError, JoinHandle},
};
use tokio_util::sync::CancellationToken;
use vessel_node::{Config, Node};
use vessel_proto::registry::WorkspaceDescriptor;

const MAX_JOBS: usize = 8;
const CLEANUP: Duration = Duration::from_secs(20);
const RELAY_CLEANUP: Duration = Duration::from_secs(5);
type Owner = JoinHandle<Option<SocketAddr>>;

#[derive(Debug, PartialEq, Eq)]
enum Timing {
    InBudget,
    Late,
}
struct Drained {
    result: Result<Option<SocketAddr>, JoinError>,
    timing: Timing,
}

#[derive(Default)]
pub(crate) struct Resources {
    stop: CancellationToken,
    relay_stop: CancellationToken,
    jobs: Vec<Owner>,
    relay: Option<Owner>,
    sources: Option<tempfile::TempDir>,
}

impl Resources {
    pub(in super::super) fn sources(&mut self) -> std::path::PathBuf {
        assert!(self.sources.is_none());
        let sources = tempfile::tempdir().unwrap();
        let path = sources.path().to_path_buf();
        self.sources = Some(sources);
        path
    }

    fn reserve(&self) {
        assert!(
            self.jobs
                .len()
                .checked_add(usize::from(self.relay.is_some()))
                .unwrap()
                < MAX_JOBS
        );
    }

    pub(in super::super) async fn node(
        &mut self,
        fixture: Arc<Fixture>,
        adjust: impl FnOnce(&mut Config) + Send + 'static,
    ) -> Arc<Node> {
        self.reserve();
        let (ready, receiver) = oneshot::channel();
        let stop = self.stop.clone();
        // Register the owner BEFORE observing startup. No timeout can discard
        // an inherited helper after it spawned a Node but before returning it.
        self.jobs.push(tokio::spawn(async move {
            let node = Arc::new(fixture.start_with(adjust).await);
            let address = node.address().unwrap();
            drop(ready.send(Arc::clone(&node)));
            stop.cancelled().await;
            let node = Arc::try_unwrap(node).expect("validation node references released");
            node.stop().await.unwrap();
            drop(fixture);
            Some(address)
        }));
        step("registry retained node startup observation", receiver)
            .await
            .unwrap()
    }

    pub(in super::super) async fn relay(&mut self) -> Arc<Relay> {
        self.reserve();
        assert!(self.relay.is_none());
        let (ready, receiver) = oneshot::channel();
        let stop = self.relay_stop.clone();
        self.relay = Some(tokio::spawn(async move {
            let relay = Arc::new(Relay::start().await);
            let address = relay
                .url
                .as_str()
                .strip_prefix("ws://")
                .unwrap()
                .parse()
                .unwrap();
            drop(ready.send(Arc::clone(&relay)));
            stop.cancelled().await;
            let relay = Arc::try_unwrap(relay)
                .unwrap_or_else(|_retained| panic!("validation relay references released"));
            relay.stop().await;
            Some(address)
        }));
        step("registry retained relay startup observation", receiver)
            .await
            .unwrap()
    }

    pub(in super::super) async fn seed(
        &mut self,
        fixture: Arc<Fixture>,
        values: Vec<WorkspaceDescriptor>,
    ) {
        self.reserve();
        let (ready, receiver) = oneshot::channel();
        self.jobs.push(tokio::spawn(async move {
            // This retained operation is never cancelled with its observer.
            // Its Store writer closes before the job can join, even on panic.
            super::super::seed::seed(&fixture, &values).await;
            let _observer_gone = ready.send(());
            None
        }));
        step("registry retained seed/commit/close observation", receiver)
            .await
            .unwrap();
    }

    pub(super) async fn close(mut self) -> Vec<SocketAddr> {
        self.stop.cancel();
        let mut joined = join_all(
            self.jobs
                .drain(..)
                .map(|job| drain("node/seed", job, CLEANUP)),
        )
        .await;
        // Keep the relay alive until all actual node shutdowns and offline
        // writers have settled, including startup jobs whose observer timed out.
        self.relay_stop.cancel();
        if let Some(relay) = self.relay.take() {
            joined.push(drain("relay", relay, RELAY_CLEANUP).await);
        }
        joined
            .into_iter()
            .filter_map(|drained| {
                assert_eq!(
                    drained.timing,
                    Timing::InBudget,
                    "registry owner cleanup exceeded reserved bound"
                );
                drained.result.unwrap()
            })
            .collect()
    }
}

async fn drain(name: &str, mut job: Owner, limit: Duration) -> Drained {
    match tokio::time::timeout(limit, &mut job).await {
        Ok(result) => Drained {
            result,
            timing: Timing::InBudget,
        },
        Err(_elapsed) => {
            // A late actual shutdown remains owned by this driver. The outer
            // observer fails its bound; abort/drop is never a normal-close ACK.
            tracing::error!(
                phase = name,
                "registry owner cleanup exceeded reserved bound"
            );
            Drained {
                result: job.await,
                timing: Timing::Late,
            }
        }
    }
}
