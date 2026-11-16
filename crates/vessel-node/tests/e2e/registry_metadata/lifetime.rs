//! A bounded test-local owner driver survives dropped validation observations.
mod resources;
use crate::support::{Fixture, step, within};
use futures_util::FutureExt as _;
pub(super) use resources::Resources;
use std::{
    io::ErrorKind,
    net::SocketAddr,
    panic::{AssertUnwindSafe, resume_unwind},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::oneshot,
    task::{JoinError, JoinHandle},
};
use tokio_util::sync::CancellationToken;

const BODY: Duration = Duration::from_secs(30);
const DRIVER: Duration = Duration::from_secs(55);

#[derive(Clone, Copy)]
pub(super) enum Case {
    Owner,
    Paired,
    Agent,
    Fleet,
    SetupFailure,
    AssertionFailure,
    ObserverDrop,
}

struct Observation(CancellationToken);
impl Drop for Observation {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

struct Driver {
    task: JoinHandle<()>,
    proof: oneshot::Receiver<Vec<SocketAddr>>,
    ready: oneshot::Receiver<()>,
}

fn start(case: Case) -> (Observation, Driver) {
    let stop = CancellationToken::new();
    let observation = Observation(stop.clone());
    let (proof, proof_rx) = oneshot::channel();
    let (ready, ready_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut resources = Resources::default();
        let result = {
            let body = AssertUnwindSafe(execute(case, &mut resources, ready)).catch_unwind();
            tokio::select! {
                result = tokio::time::timeout(BODY, body) => Some(result),
                () = stop.cancelled() => None,
            }
        };
        // The body future (and all read-only Node/Relay clones) is gone before
        // originals stop. This owner task is never aborted by observer Drop.
        let closed = resources.close().await;
        let _observer_gone = proof.send(closed);
        match result {
            Some(Ok(Ok(()))) => {}
            Some(Ok(Err(panic))) => resume_unwind(panic),
            Some(Err(_elapsed)) => panic!("registry validation body deadline"),
            None => panic!("registry validation observer dropped"),
        }
    });
    (
        observation,
        Driver {
            task,
            proof: proof_rx,
            ready: ready_rx,
        },
    )
}

impl Driver {
    async fn finish(mut self) -> (Result<(), JoinError>, Vec<SocketAddr>) {
        let result = tokio::time::timeout(DRIVER, &mut self.task)
            .await
            .expect("registry retained owner driver join deadline");
        let proof = step("registry actual owner stop/join report", self.proof)
            .await
            .unwrap();
        (result, proof)
    }
}

pub(super) async fn run(case: Case) {
    within(async move {
        let (observation, driver) = start(case);
        let (result, _closed) = driver.finish().await;
        drop(observation);
        result.unwrap();
    })
    .await;
}

async fn execute(case: Case, resources: &mut Resources, ready: oneshot::Sender<()>) {
    match case {
        Case::Owner => super::owner(resources).await,
        Case::Paired => super::authority::paired_case(resources).await,
        Case::Agent => super::authority::agent_case(resources).await,
        Case::Fleet => super::fleet::check(resources).await,
        Case::SetupFailure | Case::AssertionFailure | Case::ObserverDrop => {
            let relay = resources.relay().await;
            let fixture = Arc::new(Fixture::new());
            let node = resources.node(Arc::clone(&fixture), |_| {}).await;
            assert!(
                !matches!(case, Case::SetupFailure),
                "deliberate setup failure after actual owners started"
            );
            let client = step(
                "failure-control actual owner connect",
                fixture.connect(&node),
            )
            .await;
            assert_eq!(client.node_info().await.unwrap().id, node.info().id);
            assert!(
                !matches!(case, Case::AssertionFailure),
                "deliberate validation assertion failure"
            );
            let _observed = ready.send(());
            std::future::pending::<()>().await;
            drop(relay);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_metadata_failed_setup_assertion_and_dropped_observer_stop_actual_owners() {
    within(async {
        for case in [
            Case::SetupFailure,
            Case::AssertionFailure,
            Case::ObserverDrop,
        ] {
            let (observation, mut driver) = start(case);
            let readiness = if matches!(case, Case::ObserverDrop) {
                Some(tokio::time::timeout(crate::support::STEP, &mut driver.ready).await)
            } else {
                None
            };
            if matches!(case, Case::ObserverDrop) {
                drop(observation);
            }
            let (result, closed) = driver.finish().await;
            if let Some(readiness) = readiness {
                readiness
                    .expect("failure-control actual setup readiness deadline")
                    .unwrap();
            }
            assert!(result.unwrap_err().is_panic());
            assert_eq!(
                closed.len(),
                2,
                "one actual Node and one actual Relay joined"
            );
            for address in closed {
                let result = step(
                    "original fixture endpoint closed after actual join",
                    tokio::net::TcpStream::connect(address),
                )
                .await;
                assert_eq!(
                    result.unwrap_err().kind(),
                    ErrorKind::ConnectionRefused,
                    "original node/relay must no longer accept"
                );
            }
        }
    })
    .await;
}
