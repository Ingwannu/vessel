//! A completed registration drain leaves an already handed-off secure carrier alive.
use super::support::{Relay, tls, token};
use futures_util::{SinkExt as _, StreamExt as _};
use std::{
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use vessel_proto::RouteUrl;
use vessel_relay::{LinkState, NodeLink, WsCarrier, dial};
use vessel_secure::{Config, NodeKeypair, SecureChannel, initiate, routing_id};
const OUTER: Duration = Duration::from_secs(30);
const PHASE: Duration = Duration::from_secs(5);
const ACCEPTED: usize = 1;

struct Running {
    stop: CancellationToken,
    task: JoinHandle<()>,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.stop.cancel();
        self.task.abort();
    }
}
async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, Box::pin(future))
        .await
        .unwrap_or_else(|_elapsed| panic!("relay lifetime phase: {name}"))
}

#[tokio::test]
async fn registration_stop_acknowledges_after_handoff_without_closing_encrypted_data() {
    vessel_testkit::within(
        OUTER,
        Box::pin(async {
            let mut relay = Relay::start().await;
            let key = Arc::new(NodeKeypair::generate().unwrap());
            let (accept, mut accepted) = mpsc::channel(ACCEPTED);
            let Registration { mut running, state } =
                registered(&relay.url, &key, move |carrier| {
                    accept.try_send(carrier).unwrap();
                })
                .await;
            let EncryptedPair {
                mut client,
                mut server,
            } = encrypted_pair(&relay.url, &key, &mut accepted).await;
            phase(
                "encrypted message before disable",
                client.send("before".to_owned()),
            )
            .await
            .unwrap();
            assert_eq!(
                phase("gateway receives before disable", server.next())
                    .await
                    .unwrap()
                    .unwrap(),
                "before"
            );
            running.stop.cancel();
            phase(
                "registration and all pending opens drained",
                &mut running.task,
            )
            .await
            .unwrap();
            assert_eq!(*state.borrow(), LinkState::Stopped);
            phase(
                "encrypted message after disable acknowledgement",
                client.send("after".to_owned()),
            )
            .await
            .unwrap();
            assert_eq!(
                phase("existing gateway connection survives", server.next())
                    .await
                    .unwrap()
                    .unwrap(),
                "after"
            );
            phase(
                "reply on existing encrypted connection",
                server.send("still connected".to_owned()),
            )
            .await
            .unwrap();
            assert_eq!(
                phase("client receives after disable", client.next())
                    .await
                    .unwrap()
                    .unwrap(),
                "still connected"
            );
            assert!(accepted.try_recv().is_err());
            drop(client);
            drop(server);
            phase("isolated relay server stopped", relay.stop_owned()).await;
        }),
    )
    .await
    .expect("relay lifetime outer safety");
}

struct EncryptedPair {
    client: SecureChannel<WsCarrier>,
    server: SecureChannel<WsCarrier>,
}

async fn encrypted_pair(
    relay: &RouteUrl,
    key: &NodeKeypair,
    accepted: &mut mpsc::Receiver<WsCarrier>,
) -> EncryptedPair {
    let route_id = routing_id(key.public());
    let transport_tls = tls();
    let (client, server) = tokio::join!(
        phase(
            "client reaches registered node",
            dial(relay, &route_id, &transport_tls)
        ),
        phase("gateway receives carrier ownership", accepted.recv()),
    );
    let client = client.unwrap();
    let server = server.unwrap();
    let secure = Config::default();
    let (client, server) = tokio::join!(
        phase("client encrypted handshake", initiate(client, key.public())),
        phase(
            "gateway encrypted handshake",
            vessel_relay::respond(server, key, &secure)
        ),
    );
    let client = client.unwrap();
    let server = server.unwrap();
    EncryptedPair { client, server }
}

struct Registration {
    running: Running,
    state: watch::Receiver<LinkState>,
}

async fn registered<F>(relay: &RouteUrl, key: &Arc<NodeKeypair>, accept: F) -> Registration
where
    F: Fn(WsCarrier) + Send + Sync + 'static,
{
    let link = NodeLink::new(relay.clone(), Arc::clone(key), token(), tls());
    let mut state = link.state();
    let stop = CancellationToken::new();
    let running = Running {
        stop: stop.clone(),
        task: tokio::spawn(link.run(accept, stop)),
    };
    phase(
        "configured registration ready",
        state.wait_for(|state| *state == LinkState::Registered),
    )
    .await
    .unwrap();
    Registration { running, state }
}

struct HeldAccept {
    entry: oneshot::Receiver<()>,
    release: std::sync::mpsc::SyncSender<()>,
    finished: Arc<AtomicUsize>,
}
impl HeldAccept {
    fn prepare() -> (impl Fn(WsCarrier) + Send + Sync + 'static, Self) {
        const RELEASE_CAPACITY: usize = 1;
        let (entered, entry) = oneshot::channel();
        let (release, wait) = std::sync::mpsc::sync_channel::<()>(RELEASE_CAPACITY);
        let gate = Mutex::new((Some(entered), wait));
        let finished = Arc::new(AtomicUsize::new(0));
        let completed = Arc::clone(&finished);
        let callback = move |_carrier| {
            // Hold the actual public runner's final handoff, with a finite
            // deadline and without occupying its executor worker meanwhile.
            tokio::task::block_in_place(|| {
                let (entered, wait) = &mut *gate.lock().unwrap();
                entered.take().unwrap().send(()).unwrap();
                wait.recv_timeout(PHASE)
                    .expect("release public runner accept");
                completed.fetch_add(1, Ordering::SeqCst);
            });
        };
        (
            callback,
            Self {
                entry,
                release,
                finished,
            },
        )
    }
    async fn assert_stop_waits(&mut self, registration: &mut Registration) {
        phase("public runner accept callback entered", &mut self.entry)
            .await
            .unwrap();
        registration.running.stop.cancel();
        let pending = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(
                std::pin::Pin::new(&mut registration.running.task)
                    .poll(cx)
                    .is_pending(),
            )
        })
        .await;
        assert!(
            pending,
            "the public run must join its pending callbacks before ACK"
        );
        assert_eq!(self.finished.load(Ordering::SeqCst), 0);
        assert_ne!(*registration.state.borrow(), LinkState::Stopped);
        self.release.try_send(()).unwrap();
        phase(
            "public runner joins the released callback",
            &mut registration.running.task,
        )
        .await
        .unwrap();
        assert_eq!(self.finished.load(Ordering::SeqCst), 1);
        assert_eq!(*registration.state.borrow(), LinkState::Stopped);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_public_run_cannot_acknowledge_while_its_accept_callback_is_held() {
    vessel_testkit::within(
        OUTER,
        Box::pin(async {
            let mut relay = Relay::start().await;
            let key = Arc::new(NodeKeypair::generate().unwrap());
            let (callback, mut held) = HeldAccept::prepare();
            let mut registration = registered(&relay.url, &key, callback).await;
            let id = routing_id(key.public());
            let transport = tls();
            let carrier = phase(
                "client triggers the public runner's data open",
                dial(&relay.url, &id, &transport),
            )
            .await
            .unwrap();
            held.assert_stop_waits(&mut registration).await;
            drop(carrier);
            phase(
                "relay fixture stopped after callback drain",
                relay.stop_owned(),
            )
            .await;
        }),
    )
    .await
    .expect("public runner callback outer safety");
}
