//! Shared isolated loopback fixture; no product listener or real credentials.
use axum::Router;
use std::time::Duration;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use vessel_proto::dictation::EndpointBase;

const CLEANUP_DEADLINE: Duration = Duration::from_secs(5);

pub(super) const SYNTHETIC_KEY: &str = "voice-fixture-credential";

pub(super) struct Server {
    pub(super) base: EndpointBase,
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
}
impl Server {
    pub(super) async fn start(router: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let stop = CancellationToken::new();
        let closing = stop.clone();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(closing.cancelled_owned())
                .await
                .unwrap();
        });
        Self {
            base: format!("http://{address}/v1").parse().unwrap(),
            stop,
            task: Some(task),
        }
    }
    pub(super) async fn finish(mut self) {
        self.stop.cancel();
        // Keep ownership in self while waiting: if the finite wait panics,
        // Drop still aborts this fixture rather than detaching the server.
        tokio::time::timeout(CLEANUP_DEADLINE, self.task.as_mut().unwrap())
            .await
            .unwrap()
            .unwrap();
        drop(self.task.take());
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
