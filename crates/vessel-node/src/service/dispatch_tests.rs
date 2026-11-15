//! A call never builds a large future on a tokio worker's stack.
//!
//! Gate 49b (2026-11-15) aborted the CLI and node e2e tests with a worker
//! stack overflow. In debug builds the per-call dispatch future, sized by
//! the largest guarded handler, outgrew tokio's 2 MiB default worker stack
//! once every handler had the diagnostics wrapper beside batch 11's
//! handlers. The guard now boxes each call (`guard.rs`). This test measures
//! the real dispatcher's future, then answers real calls (diagnostics, the
//! heaviest of them, included) through the node's guarded service on
//! default-stack workers.

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use vessel_client::NodeClient;
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::{ClientInfo, GetNode, Request as _};
use vessel_rpc::{ClientConfig, Server, memory};
use vessel_store::DataDir;

use super::Connection;
use crate::principal::Principal;
use crate::{Config, Node};

/// The most a dispatch future may take on a stack: a boxed call is a
/// pointer plus its request, so this is far above what it needs and far
/// below the 2 MiB a worker has.
const DISPATCH_FUTURE_LIMIT: usize = 64 * 1024;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_dispatch_future_is_small_and_calls_run_on_default_worker_stacks() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = Config::new(
        DataDir::new(temp.path().join("data")),
        HarnessEnv::new(temp.path().join("user")),
    );
    config.bind = None;
    config.harness_lookup = Lookup::new(Vec::<PathBuf>::new());
    let node = Node::start(config).await.unwrap();
    let gateway = &node.secure;
    let guarded = Arc::clone(&gateway.service);
    let server = Server::new(
        Arc::clone(&gateway.service),
        gateway.lifecycle.watch_info(),
        gateway.config.clone(),
        CancellationToken::new(),
    );
    let (client_end, node_end) = memory::pair(NonZeroUsize::new(16).unwrap());
    let (made, contexts) = oneshot::channel::<Connection>();
    let serving = tokio::spawn(async move {
        server
            .serve(node_end, move |peer| {
                let connection = Connection::owner(peer);
                drop(made.send(connection.acting_as(Principal::Owner)));
                connection
            })
            .await
    });
    let client = NodeClient::over(
        client_end,
        ClientInfo {
            name: "dispatch-test".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    let kept = contexts.await.unwrap();

    // One type for every call: its size is the largest arm's.
    let size = size_of_val(&vessel_proto::dispatch(
        &*guarded,
        &kept,
        GetNode.into_call(),
    ));
    assert!(
        size <= DISPATCH_FUTURE_LIMIT,
        "the dispatch future takes {size} bytes"
    );

    // Real calls, answered by tasks on the runtime's default-stack workers.
    let info = client.call(GetNode).await.unwrap();
    assert_eq!(info.id, client.node().id);
    let report = client.diagnostics().await.unwrap();
    assert_eq!(report.fields().node.id, info.id);

    drop(kept);
    drop(client);
    serving.await.unwrap().unwrap();
    node.stop().await.unwrap();
}
