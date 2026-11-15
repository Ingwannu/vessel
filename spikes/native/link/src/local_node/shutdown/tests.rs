//! Synthetic ownership verifies runtime handoff without launching a real agent.

use super::*;
use std::sync::{Arc, mpsc as sync};
use std::time::Duration;
use tokio::runtime::{Builder, Handle};

const TEST_DEADLINE: Duration = Duration::from_secs(10);

struct OwnedDrop(sync::Sender<()>);
impl Drop for OwnedDrop {
    fn drop(&mut self) {
        let _closed = self.0.send(());
    }
}

fn snapshot(status: Status) -> Snapshot {
    Snapshot {
        status,
        failure: None,
        connection: None,
        permit: Arc::new(()),
    }
}

struct Fixture {
    policy: Option<sync::SyncSender<Policy>>,
    requests: sync::Receiver<oneshot::Sender<Result<(), ManagerError>>>,
    published: watch::Sender<Snapshot>,
    dropped: sync::Receiver<()>,
    finished: sync::Receiver<()>,
    reactor: Handle,
    supervisor: std::thread::JoinHandle<()>,
}

impl Fixture {
    fn new() -> Self {
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let reactor = runtime.handle().clone();
        let (policy, admission) = sync::sync_channel(CAPACITY);
        let (commands, mut inbox) = mpsc::channel(1);
        let (published, state) =
            watch::channel(snapshot(Status::Kept(vessel_proto::NodeId::random())));
        let (drop_notice, dropped) = sync::channel();
        let (request_notice, requests) = sync::channel();
        let (done, finished) = sync::channel();
        let supervisor = std::thread::spawn(move || {
            let owned = OwnedDrop(drop_notice);
            drop(runtime.spawn(async move {
                let _owned = owned;
                std::future::pending::<()>().await;
            }));
            drop(runtime.spawn(async move {
                if let Some(actor::Command::PreserveExit(reply)) = inbox.recv().await {
                    request_notice.send(reply).unwrap();
                }
            }));
            run(runtime, &admission, commands, state);
            done.send(()).unwrap();
        });
        Self {
            policy: Some(policy),
            requests,
            published,
            dropped,
            finished,
            reactor,
            supervisor,
        }
    }

    fn ping(&self) {
        let (pong, receive) = sync::channel();
        drop(self.reactor.spawn(async move {
            pong.send(()).unwrap();
        }));
        receive.recv_timeout(TEST_DEADLINE).unwrap();
    }

    fn drop_ui(&mut self) {
        let policy = self.policy.take().unwrap();
        let _later_drop = policy.try_send(Policy::Ordinary);
        drop(policy);
    }

    fn join(self) {
        self.finished.recv_timeout(TEST_DEADLINE).unwrap();
        self.dropped.recv_timeout(TEST_DEADLINE).unwrap();
        self.supervisor.join().unwrap();
    }
}

#[test]
fn ordinary_owner_drop_releases_the_reactor_and_its_owned_tasks() {
    let mut fixture = Fixture::new();
    fixture.drop_ui();
    fixture.join();
}

#[test]
fn irreversible_exit_survives_ui_drop_while_release_ack_is_still_pending() {
    let mut fixture = Fixture::new();
    fixture
        .policy
        .as_ref()
        .unwrap()
        .try_send(Policy::Preserve)
        .unwrap();
    let acknowledgement = fixture.requests.recv_timeout(TEST_DEADLINE).unwrap();
    // Later ordinary Controller::drop cannot overtake the handed-off policy.
    fixture.drop_ui();
    fixture.ping();
    assert!(fixture.dropped.try_recv().is_err());
    acknowledgement.send(Ok(())).unwrap();
    fixture.ping();
    // The direct-child observer/reaper remains alive after surrender as well.
    fixture
        .published
        .send_modify(|snapshot| snapshot.status = Status::Stopped);
    fixture.join();
}

#[test]
fn attached_successor_does_not_skip_an_original_released_child_acknowledgement() {
    let mut fixture = Fixture::new();
    fixture.published.send_modify(|snapshot| {
        snapshot.status = Status::Attached(vessel_proto::NodeId::random());
    });
    fixture
        .policy
        .as_ref()
        .unwrap()
        .try_send(Policy::Preserve)
        .unwrap();
    let original_children_reaped = fixture.requests.recv_timeout(TEST_DEADLINE).unwrap();
    fixture.drop_ui();
    fixture.ping();
    assert!(fixture.finished.try_recv().is_err());
    assert!(fixture.dropped.try_recv().is_err());
    original_children_reaped.send(Ok(())).unwrap();
    fixture.join();
}
