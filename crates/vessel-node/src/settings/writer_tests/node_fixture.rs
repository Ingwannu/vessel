//! Retain each real Node through failed setup/body observation and normal drain.
//! This owns test resources only; no product admission or cleanup is replaced.

use std::future::Future;
use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use futures_util::{FutureExt as _, SinkExt as _};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Instant, error::Elapsed};
use tokio_tungstenite::tungstenite;
use tokio_util::task::TaskTracker;

pub(in crate::settings) type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

#[derive(Clone, Copy)]
pub(in crate::settings) struct Budget {
    case: Duration,
    phase: Duration,
}

impl Budget {
    pub(in crate::settings) fn new(case: Duration, phase: Duration) -> Self {
        // Reserve half a phase for normal drain, without increasing either
        // existing case limit. Body observation expires before the case clock.
        let budget = Self { case, phase };
        assert!(phase < case.checked_sub(budget.drain()).unwrap());
        budget
    }

    fn drain(self) -> Duration {
        self.phase.checked_div(2).unwrap()
    }
}

#[derive(Clone, Copy)]
pub(in crate::settings) enum Gate {
    Write,
    Place,
}

#[derive(Default)]
struct Releases {
    write: Option<oneshot::Sender<()>>,
    place: Option<oneshot::Sender<()>>,
}

pub(in crate::settings) struct Resources {
    pub(in crate::settings) address: Option<SocketAddr>,
    pub(in crate::settings) service: Arc<crate::guard::Guarded>,
    pub(in crate::settings) tasks: TaskTracker,
    pub(in crate::settings) socket: tokio::sync::Mutex<Option<Socket>>,
    client: Mutex<Option<vessel_client::NodeClient>>,
    releases: Mutex<Releases>,
}

impl Resources {
    pub(in crate::settings) fn retain_client(&self, client: &vessel_client::NodeClient) {
        let mut slot = self.client.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(slot.replace(client.clone()).is_none());
    }

    pub(in crate::settings) fn retain(&self, gate: Gate, release: oneshot::Sender<()>) {
        let mut releases = self.releases.lock().unwrap_or_else(PoisonError::into_inner);
        let slot = match gate {
            Gate::Write => &mut releases.write,
            Gate::Place => &mut releases.place,
        };
        assert!(slot.replace(release).is_none());
    }

    pub(in crate::settings) fn release(&self, gate: Gate) -> Result<(), ()> {
        let release = {
            let mut releases = self.releases.lock().unwrap_or_else(PoisonError::into_inner);
            match gate {
                Gate::Write => releases.write.take(),
                Gate::Place => releases.place.take(),
            }
        };
        release
            .expect("the fixture registered this exact gate")
            .send(())
    }

    fn release_all(&self) {
        let mut releases = self.releases.lock().unwrap_or_else(PoisonError::into_inner);
        for release in [releases.write.take(), releases.place.take()]
            .into_iter()
            .flatten()
        {
            // A receiver already gone needs no second action. This is a gate
            // release, never evidence that its actual worker has completed.
            let _released_or_receiver_gone = release.send(());
        }
    }

    pub(in crate::settings) async fn close_socket(&self) -> Result<(), tungstenite::Error> {
        let mut socket = self.socket.lock().await;
        let result = match socket.as_mut() {
            Some(socket) => socket.close(None).await,
            None => Ok(()),
        };
        // Drop the owned transport even on a close error; normal Node shutdown
        // separately joins its server-side connection before returning.
        drop(socket.take());
        result
    }
}

pub(in crate::settings) enum Body {
    Passed,
    TimedOut(Elapsed),
    StartupTimedOut(Elapsed),
    ObserverLost,
}

pub(in crate::settings) struct Settled {
    pub(in crate::settings) body: std::thread::Result<Body>,
    pub(in crate::settings) resources: Arc<Resources>,
    stopped: Result<(), crate::NodeError>,
    socket: Result<(), tungstenite::Error>,
    drain_expired: Option<Elapsed>,
    case_expired: Option<Elapsed>,
}

impl Settled {
    pub(in crate::settings) async fn assert_drained(&self) {
        assert!(
            self.drain_expired.is_none(),
            "actual drain exceeded its named observation phase"
        );
        assert!(
            self.stopped.is_ok(),
            "actual node.stop did not finish successfully"
        );
        assert!(self.resources.tasks.is_empty(), "fixture workers remain");
        let settings = self.resources.service.0.settings_for_test();
        assert!(matches!(
            settings
                .state
                .recovery
                .store()
                .set_restart_policy(false)
                .await,
            Err(vessel_store::StoreError::Closed)
        ));
    }

    pub(in crate::settings) fn assert_observed(&self) {
        assert!(self.case_expired.is_none(), "fixture case observer expired");
    }

    pub(in crate::settings) fn assert_success(self) {
        // Preserve the original body panic, but only after physical cleanup.
        let body = self
            .body
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        match body {
            Body::Passed => {}
            Body::TimedOut(error) => panic!("fixture body: {error}"),
            Body::StartupTimedOut(error) => panic!("actual node startup: {error}"),
            Body::ObserverLost => panic!("fixture body observer was lost"),
        }
        assert!(self.case_expired.is_none(), "fixture case observer expired");
        assert!(
            self.drain_expired.is_none(),
            "normal node/worker drain phase expired"
        );
        self.socket.unwrap();
        self.stopped.unwrap();
    }
}

pub(in crate::settings) struct Running {
    worker: JoinHandle<Settled>,
    observing: Option<oneshot::Sender<()>>,
    started: Instant,
    budget: Budget,
}

impl Running {
    pub(in crate::settings) fn start<F, Fut>(config: crate::Config, budget: Budget, body: F) -> Self
    where
        F: FnOnce(Arc<Resources>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Match within's preparation boundary before any real Node exists.
        // within repeats only the cached check when observing this worker.
        vessel_testkit::prepare().unwrap();
        let started = Instant::now();
        let (observing, lost) = oneshot::channel();
        let worker = tokio::spawn(Box::pin(drive(config, budget, started, lost, body)));
        Self {
            worker,
            observing: Some(observing),
            started,
            budget,
        }
    }

    pub(in crate::settings) fn abandon_observer(&mut self) {
        drop(self.observing.take());
    }

    pub(in crate::settings) async fn finish(mut self) -> Settled {
        let remaining = self.budget.case.saturating_sub(self.started.elapsed());
        match vessel_testkit::within(remaining, &mut self.worker).await {
            Ok(settled) => settled.unwrap(),
            Err(error) => {
                self.abandon_observer();
                // The deadline records failure, not physical completion. Keep
                // this exact driver until real stop and tracked work settle.
                let mut settled = self.worker.await.unwrap();
                settled.case_expired = Some(error);
                settled
            }
        }
    }
}

async fn drive<F, Fut>(
    config: crate::Config,
    budget: Budget,
    started: Instant,
    mut lost: oneshot::Receiver<()>,
    body: F,
) -> Settled
where
    F: FnOnce(Arc<Resources>) -> Fut,
    Fut: Future<Output = ()>,
{
    let (node, startup_expired) = start_node(config, budget).await;
    let resources = Arc::new(Resources {
        address: node.address(),
        service: Arc::clone(&node.secure.service),
        tasks: TaskTracker::new(),
        socket: tokio::sync::Mutex::new(None),
        client: Mutex::new(None),
        releases: Mutex::new(Releases::default()),
    });
    let outcome = match startup_expired {
        Some(error) => Ok(Body::StartupTimedOut(error)),
        None => {
            Box::pin(observe_body(
                budget,
                started,
                &mut lost,
                Arc::clone(&resources),
                body,
            ))
            .await
        }
    };
    let ((socket, stopped), drain_expired) = drain(node, &resources, budget).await;
    Settled {
        body: outcome,
        resources,
        stopped,
        socket,
        drain_expired,
        case_expired: None,
    }
}

async fn start_node(config: crate::Config, budget: Budget) -> (crate::Node, Option<Elapsed>) {
    // Do not cancel partially completed startup: retain its result so any Node
    // actually created can stop normally even after a failed observation.
    let mut starting = Box::pin(crate::Node::start(config));
    match tokio::time::timeout(budget.phase, &mut starting).await {
        Ok(node) => (node.unwrap(), None),
        Err(error) => (starting.await.unwrap(), Some(error)),
    }
}

async fn observe_body<F, Fut>(
    budget: Budget,
    started: Instant,
    lost: &mut oneshot::Receiver<()>,
    resources: Arc<Resources>,
    body: F,
) -> std::thread::Result<Body>
where
    F: FnOnce(Arc<Resources>) -> Fut,
    Fut: Future<Output = ()>,
{
    let body_end = started
        .checked_add(budget.case.checked_sub(budget.drain()).unwrap())
        .unwrap();
    let observing = AssertUnwindSafe(tokio::time::timeout_at(body_end, async move {
        body(resources).await;
    }))
    .catch_unwind();
    tokio::select! {
        biased;
        _ = lost => Ok(Body::ObserverLost),
        result = observing => result.map(|result| match result {
            Ok(()) => Body::Passed,
            Err(error) => Body::TimedOut(error),
        }),
    }
}

type Drain = (Result<(), tungstenite::Error>, Result<(), crate::NodeError>);

async fn drain(
    node: crate::Node,
    resources: &Resources,
    budget: Budget,
) -> (Drain, Option<Elapsed>) {
    resources
        .service
        .0
        .lifecycle
        .stop(vessel_proto::StopWhen::Now);
    resources.release_all();
    let _first_close = resources.tasks.close();
    let mut physical = Box::pin(async {
        let socket = resources.close_socket().await;
        let (stopped, ()) = tokio::join!(node.stop(), resources.tasks.wait());
        let client = resources
            .client
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(client) = client {
            client.closed().await;
        }
        (socket, stopped)
    });
    match tokio::time::timeout(budget.drain(), &mut physical).await {
        Ok(result) => (result, None),
        // Preserve a named failing observation without dropping the real stop
        // future. A timeout is never a successful drain or capacity receipt.
        Err(error) => (physical.await, Some(error)),
    }
}

#[cfg(test)]
mod tests;
