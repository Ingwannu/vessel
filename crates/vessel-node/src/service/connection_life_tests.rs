//! A real connection's context carries one life (`connection_life`): every
//! context acting for the connection shares its owner, and the transport
//! closing ends the life even while a context is still held, as a call
//! still running in the close grace holds one.

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use vessel_client::NodeClient;
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::ClientInfo;
use vessel_rpc::{ClientConfig, Server, memory};
use vessel_store::DataDir;

use super::Connection;
use crate::connection_life::Owner;
use crate::principal::Principal;
use crate::{Config, Node};

/// How long the node may take to notice its transport closing and end the
/// life: it takes milliseconds, so this only bounds a hang.
const CLOSE_SEEN_WITHIN: Duration = Duration::from_secs(10);

#[tokio::test]
async fn a_connections_contexts_share_one_life_that_ends_with_its_transport() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = Config::new(
        DataDir::new(temp.path().join("data")),
        HarnessEnv::new(temp.path().join("user")),
    );
    config.bind = None;
    config.harness_lookup = Lookup::new(Vec::<PathBuf>::new());
    let node = Node::start(config).await.unwrap();
    // The node's own guarded service, served over an in-memory pair the
    // way its gateways serve a socket.
    let gateway = &node.secure;
    let server = Server::new(
        Arc::clone(&gateway.service),
        gateway.lifecycle.watch_info(),
        gateway.config.clone(),
        CancellationToken::new(),
    );
    let (client_end, node_end) = memory::pair(NonZeroUsize::new(16).unwrap());
    let (made, contexts) = oneshot::channel::<(Owner, Connection)>();
    let serving = tokio::spawn(async move {
        server
            .serve(node_end, move |peer| {
                let connection = Connection::owner(peer);
                let kept = connection.acting_as(Principal::Owner);
                drop(made.send((connection.life().owner(), kept)));
                connection
            })
            .await
    });
    let client = NodeClient::over(
        client_end,
        ClientInfo {
            name: "life-test".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    let (served, kept) = contexts.await.unwrap();
    let owner = kept.life().owner();
    let closed = kept.life().closed();
    assert_eq!(
        owner, served,
        "a context acting for the connection shares its owner"
    );
    assert!(!closed.is_cancelled(), "the connection is open");

    drop(client);
    serving.await.unwrap().unwrap();
    tokio::time::timeout(CLOSE_SEEN_WITHIN, closed.cancelled())
        .await
        .expect("the transport closing ends the life while a context is held");
    assert_eq!(kept.life().owner(), owner, "the identity outlives the life");
    drop(kept);
    node.stop().await.unwrap();
}
