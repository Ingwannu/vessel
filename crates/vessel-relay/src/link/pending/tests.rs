//! Owner-level barriers; actual websocket/secure survival is tested separately.
use super::*;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
const OUTER: Duration = Duration::from_secs(30);
const PHASE: Duration = Duration::from_secs(5);
const CALLBACK_RELEASE_CAPACITY: usize = 1;

async fn within(future: impl Future<Output = ()>) {
    vessel_testkit::within(OUTER, Box::pin(future))
        .await
        .expect("pending data outer safety");
}
async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, Box::pin(future))
        .await
        .unwrap_or_else(|_elapsed| panic!("pending data phase: {name}"))
}
struct Dropped(Arc<AtomicUsize>);
impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn capacity_is_reserved_before_factory_and_stop_joins_all_pending_drops() {
    within(async {
        let registration = CancellationToken::new();
        let mut data = DataOpens::new(&registration);
        let created = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicUsize::new(0));
        let (ready, mut readiness) = mpsc::channel(MAX_PENDING_DATA_OPENS);
        for _ in 0..MAX_PENDING_DATA_OPENS {
            let (created, dropped, ready) =
                (Arc::clone(&created), Arc::clone(&dropped), ready.clone());
            assert_eq!(
                data.start(move |_stop| async move {
                    created.fetch_add(1, Ordering::SeqCst);
                    let _drop = Dropped(dropped);
                    ready.send(()).await.unwrap();
                    std::future::pending::<()>().await;
                }),
                Admission::Accepted
            );
        }
        for _ in 0..MAX_PENDING_DATA_OPENS {
            phase("pending dial started", readiness.recv())
                .await
                .unwrap();
        }
        assert_eq!(created.load(Ordering::SeqCst), MAX_PENDING_DATA_OPENS);
        let extra = Arc::clone(&created);
        assert_eq!(
            data.start(move |_stop| {
                extra.fetch_add(1, Ordering::SeqCst);
                async {}
            }),
            Admission::Full
        );
        assert_eq!(created.load(Ordering::SeqCst), MAX_PENDING_DATA_OPENS);
        registration.cancel();
        phase("all pending children cancelled and joined", data.stop()).await;
        assert_eq!(dropped.load(Ordering::SeqCst), MAX_PENDING_DATA_OPENS);
        assert!(data.tasks.is_empty());
        assert_eq!(data.start(|_| async {}), Admission::Stopping);
    })
    .await;
}

#[tokio::test]
async fn completed_children_release_capacity_only_after_reaping() {
    within(async {
        let registration = CancellationToken::new();
        let mut data = DataOpens::new(&registration);
        let (send, done) = oneshot::channel();
        assert_eq!(
            data.start(move |_| async move {
                send.send(()).unwrap();
            }),
            Admission::Accepted
        );
        phase("completed child body", done).await.unwrap();
        // Reap the actual task completion, not only its last body statement.
        phase("completed child joined", data.tasks.join_next())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(data.start(|_| async {}), Admission::Accepted);
        phase("stop after capacity reuse", data.stop()).await;
        assert!(data.tasks.is_empty());
    })
    .await;
}

#[tokio::test]
async fn dropping_the_run_cancels_and_aborts_its_children() {
    within(async {
        let registration = CancellationToken::new();
        let mut data = DataOpens::new(&registration);
        let (ready, started) = oneshot::channel();
        let (gone, dropped) = oneshot::channel();
        struct Notice(Option<oneshot::Sender<()>>);
        impl Drop for Notice {
            fn drop(&mut self) {
                if let Some(send) = self.0.take() {
                    let _gone = send.send(());
                }
            }
        }
        assert_eq!(
            data.start(move |_| async move {
                let _notice = Notice(Some(gone));
                ready.send(()).unwrap();
                std::future::pending::<()>().await;
            }),
            Admission::Accepted
        );
        phase("child entered before run drop", started)
            .await
            .unwrap();
        drop(data);
        phase("run drop aborts owned child", dropped).await.unwrap();
        assert!(
            !registration.is_cancelled(),
            "dropping a run must not cancel a caller's broader shutdown token"
        );
    })
    .await;
}

#[tokio::test]
async fn an_actual_websocket_upgrade_is_dropped_before_stop_returns() {
    within(async {
        use tokio::{
            io::{AsyncBufReadExt as _, AsyncReadExt as _, BufReader},
            net::TcpListener,
        };
        const HEADER_BYTES: u64 = 4096;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let route: RouteUrl = format!("ws://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let registration = CancellationToken::new();
        let mut data = DataOpens::new(&registration);
        let accepted = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&accepted);
        data.open(
            &route,
            &TlsConfig::with_roots(rustls::RootCertStore::empty()).unwrap(),
            Reach::Any,
            ConnId::from_bytes([7; 16]),
            Arc::new(move |_carrier| {
                count.fetch_add(1, Ordering::SeqCst);
            }),
        );
        let (peer, _) = phase("data dial reaches the loopback listener", listener.accept())
            .await
            .unwrap();
        let mut peer = BufReader::new(peer.take(HEADER_BYTES));
        let mut line = Vec::new();
        loop {
            line.clear();
            let read = phase(
                "data websocket upgrade headers",
                peer.read_until(b'\n', &mut line),
            )
            .await
            .unwrap();
            assert!(read > 0);
            if line == b"\r\n" {
                break;
            }
        }
        // The peer deliberately withholds its 101 response: the real socket
        // future is awaiting handshake when registration cancellation arrives.
        registration.cancel();
        phase("cancel and reap the real pending handshake", data.stop()).await;
        let mut byte = [0_u8; 1];
        assert_eq!(
            phase(
                "pending socket closes before acknowledgement",
                peer.read(&mut byte)
            )
            .await
            .unwrap(),
            0
        );
        assert_eq!(accepted.load(Ordering::SeqCst), 0);
        assert!(data.tasks.is_empty());
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_waits_for_a_final_callback_already_in_progress() {
    within(async {
        let registration = CancellationToken::new();
        let mut data = DataOpens::new(&registration);
        let (entered, entry) = oneshot::channel();
        let (release, wait) = std::sync::mpsc::sync_channel::<()>(CALLBACK_RELEASE_CAPACITY);
        let finished = Arc::new(AtomicUsize::new(0));
        let completed = Arc::clone(&finished);
        assert_eq!(
            data.start(move |_| async move {
                // Only the test holds a synchronous callback open. block_in_place
                // releases the executor worker; production's callback only hands off.
                tokio::task::block_in_place(|| {
                    entered.send(()).unwrap();
                    wait.recv_timeout(PHASE).expect("release final callback");
                    completed.fetch_add(1, Ordering::SeqCst);
                });
            }),
            Admission::Accepted
        );
        phase("final callback entered", entry).await.unwrap();
        registration.cancel();
        let mut stopping = Box::pin(data.stop());
        let pending = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(stopping.as_mut().poll(cx).is_pending())
        })
        .await;
        assert!(
            pending,
            "a token check alone cannot acknowledge this callback"
        );
        assert_eq!(finished.load(Ordering::SeqCst), 0);
        release.try_send(()).unwrap();
        phase(
            "final callback joined before stop acknowledgement",
            stopping,
        )
        .await;
        assert_eq!(finished.load(Ordering::SeqCst), 1);
        assert!(data.tasks.is_empty());
    })
    .await;
}
