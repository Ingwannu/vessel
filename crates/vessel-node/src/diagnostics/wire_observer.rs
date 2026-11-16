//! One-shot test observation around the real guarded future, never a product hook.
//! Delegation comes from the canonical table, not a second list of RPC methods.
use super::deadline::phase;
use crate::{Node, guard::Guarded, service::Connection};
use std::{
    sync::{Arc, Mutex},
    task::Poll,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use vessel_proto::diagnostics::{DiagnosticReport, GetDiagnostics};
use vessel_proto::{ClientInfo, Handler as _, WireError};
use vessel_rpc::{ClientConfig, Server, ServerConfig};

const EVENT_CAPACITY: usize = 2;
const FRAME_CAPACITY: usize = 4;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    Entered,
    Cancelled,
    Completed,
}
#[derive(Debug)]
struct DropStage {
    events: mpsc::Sender<Stage>,
    stage: Stage,
}
impl Drop for DropStage {
    fn drop(&mut self) {
        // Failure cleanup may already have dropped the observer. No panic in
        // Drop: a missing event fails the receiver's named phase deadline.
        let _closed_or_full = self.events.try_send(self.stage);
    }
}
#[derive(Debug)]
struct Observed {
    guarded: Arc<Guarded>,
    // A one-use test signal, taken under a short lock before any await. No
    // request data is retained, and following calls delegate without a signal.
    once: Mutex<Option<mpsc::Sender<Stage>>>,
}
impl Observed {
    async fn first(
        &self,
        context: &Connection,
        request: GetDiagnostics,
    ) -> Result<DiagnosticReport, WireError> {
        let sender = { self.once.lock().unwrap().take() };
        let Some(events) = sender else {
            return self.guarded.diagnostics(context, request).await;
        };
        let mut drop_stage = DropStage {
            events,
            stage: Stage::Cancelled,
        };
        let mut call = Box::pin(self.guarded.diagnostics(context, request));
        // With the actual blocking pool occupied, this first real guard poll
        // admits the source leases and queues the real metadata/Store jobs.
        assert!(std::future::poll_fn(|cx| Poll::Ready(call.as_mut().poll(cx).is_pending())).await);
        drop_stage.events.try_send(Stage::Entered).unwrap();
        let result = call.await;
        drop_stage.stage = Stage::Completed;
        result
    }
}
macro_rules! observed_method {
    (Diagnostics, $handler:ident, $request:ty, $response:ty) => {
        async fn $handler(
            &self,
            context: &Connection,
            request: $request,
        ) -> Result<$response, WireError> {
            self.first(context, request).await
        }
    };
    ($variant:ident, $handler:ident, $request:ty, $response:ty) => {
        async fn $handler(
            &self,
            context: &Connection,
            request: $request,
        ) -> Result<$response, WireError> {
            self.guarded.$handler(context, request).await
        }
    };
}
macro_rules! observed {
    ($( $(#[$doc:meta])* $variant:ident / $handler:ident / $wire:literal
        [$scope:tt, $since:tt, $shape:ident]: $request:ty => $response:ty; )*) => {
        impl vessel_proto::Handler for Observed {
            type Context = Connection;
            $( observed_method!($variant, $handler, $request, $response); )*
        }
    };
}
vessel_proto::method_table!(observed);

pub(super) struct Transport {
    pub(super) client: vessel_client::NodeClient,
    events: mpsc::Receiver<Stage>,
    shutdown: CancellationToken,
    task: tokio::task::JoinHandle<
        Result<(), vessel_rpc::ServeError<vessel_rpc::memory::Disconnected>>,
    >,
}
impl Transport {
    pub(super) async fn stage(&mut self, expected: Stage) {
        assert_eq!(
            phase("actual guarded call stage", self.events.recv()).await,
            Some(expected)
        );
    }
    pub(super) async fn close(self) {
        self.shutdown.cancel();
        drop(self.client);
        phase("observed RPC transport closes", self.task)
            .await
            .unwrap()
            .unwrap();
    }
}
pub(super) async fn transport(node: &Node) -> Transport {
    let (events, receive) = mpsc::channel(EVENT_CAPACITY);
    let observed = Arc::new(Observed {
        guarded: Arc::clone(&node.secure.service),
        once: Mutex::new(Some(events)),
    });
    let shutdown = CancellationToken::new();
    let server = Server::new(
        observed,
        node.secure.lifecycle.watch_info(),
        ServerConfig::default(),
        shutdown.clone(),
    );
    let (client_end, server_end) =
        vessel_rpc::memory::pair(std::num::NonZeroUsize::new(FRAME_CAPACITY).unwrap());
    let task = tokio::spawn(async move { server.serve(server_end, Connection::owner).await });
    let client = phase(
        "observed RPC welcome",
        vessel_client::NodeClient::over(
            client_end,
            ClientInfo {
                name: "diagnostic-cancel-test".to_owned(),
                version: "0".to_owned(),
            },
            &ClientConfig::default(),
        ),
    )
    .await
    .unwrap();
    Transport {
        client,
        events: receive,
        shutdown,
        task,
    }
}
