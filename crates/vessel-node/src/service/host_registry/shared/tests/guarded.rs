//! Exercise the actual guard/service/actor chain while wire admission remains pending.
//! Test principals stand in for already-authenticated contexts; this is not credential proof.
use crate::{
    Config, Node, host_registry::TrustedDestinations, principal::Principal, service::Connection,
};
use std::{num::NonZeroUsize, path::PathBuf, time::Duration};
use tokio::{
    sync::{oneshot, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    ClientInfo, DeviceId, ErrorCode, Handler, NodeId, Scope, SessionId,
    host_registry::{
        AddHostRegistration, HostDestination, HostMetadata, HostRegistrationId, HostSource,
        ListHostRegistrations, ListSharedHostRegistrations,
    },
};
use vessel_store::DataDir;

#[tokio::test]
async fn guarded_public_inventory_reads_across_sources_without_widening_private_management() {
    vessel_testkit::within(Duration::from_secs(60), async {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path().join("data"));
        let mut config = Config::new(
            data.clone(),
            vessel_harness::HarnessEnv::new(root.path().join("user")),
        );
        config.bind = None;
        config.harness_lookup = vessel_harness::Lookup::new(Vec::<PathBuf>::new());
        let node = Node::start(config).await.unwrap();
        let device = DeviceId::random();
        let expected = seed(&node, device).await;
        let before = data.read_host_registry().unwrap();
        let (client, context, closing, serving) = context(&node).await;
        let reader = public_read(&node, &context, &expected).await;
        private_reads(&node, &context, &reader, device).await;
        refused_readers(&node, &context).await;
        assert_eq!(
            data.read_host_registry().unwrap().entries(),
            before.entries()
        );
        drop(client);
        closing.cancel();
        serving.await.unwrap();
        node.stop().await.unwrap();
    })
    .await
    .unwrap();
}

async fn seed(node: &Node, device: DeviceId) -> Vec<HostRegistrationId> {
    let registry = node.secure.service.0.host_registry.as_ref().unwrap();
    let destination = HostDestination::Fleet {
        node: NodeId::random(),
    };
    let mut ids = Vec::new();
    for source in [HostSource::Owner, HostSource::Device { device }] {
        let row = registry
            .add(
                source,
                AddHostRegistration {
                    destination,
                    label: Some("public worker".parse().unwrap()),
                    metadata: HostMetadata::default(),
                },
                TrustedDestinations(vec![destination]),
            )
            .await
            .unwrap();
        ids.push(row.registration_id);
    }
    ids
}

/// A real memory handshake creates Outbound/Peer; no fake protocol number is assigned.
async fn context(node: &Node) -> (NodeClient, Connection, CancellationToken, JoinHandle<()>) {
    let closing = CancellationToken::new();
    let (_state, info) = watch::channel(node.secure.lifecycle.info());
    let server = vessel_rpc::Server::new(
        node.secure.service.clone(),
        info,
        vessel_rpc::ServerConfig::default(),
        closing.clone(),
    );
    let (client_end, server_end) = vessel_rpc::memory::pair(NonZeroUsize::new(8).unwrap());
    let (sent, received) = oneshot::channel();
    let serving = tokio::spawn(async move {
        server
            .serve(server_end, |peer| {
                let context = Connection::owner(peer);
                let _delivered = sent.send(context.acting_as(Principal::Owner));
                context
            })
            .await
            .unwrap();
    });
    let client = NodeClient::over(
        client_end,
        ClientInfo {
            name: "host-public-test".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    (client, received.await.unwrap(), closing, serving)
}

async fn public_read(
    node: &Node,
    context: &Connection,
    expected: &[HostRegistrationId],
) -> Connection {
    let local = node.secure.lifecycle.info().id;
    let reader = context.acting_as(Principal::Agent {
        node: local,
        session: SessionId::random(),
        scopes: vec![Scope::Read],
    });
    let public = node
        .secure
        .service
        .list_shared_host_registrations(&reader, ListSharedHostRegistrations)
        .await
        .unwrap();
    assert_eq!(public.node, local);
    assert_eq!(
        public
            .hosts
            .entries()
            .iter()
            .map(|row| row.registration_id)
            .collect::<Vec<_>>()
            .as_slice(),
        expected
    );
    assert!(
        public
            .hosts
            .entries()
            .iter()
            .all(|row| row.destination_name.is_none())
    );
    reader
}

async fn private_reads(node: &Node, context: &Connection, reader: &Connection, device: DeviceId) {
    let private = node
        .secure
        .service
        .list_host_registrations(reader, ListHostRegistrations)
        .await
        .unwrap_err();
    assert_eq!(private.code, ErrorCode::Forbidden);
    let owner_rows = node
        .secure
        .service
        .list_host_registrations(context, ListHostRegistrations)
        .await
        .unwrap();
    assert_eq!(owner_rows.hosts.len(), 1);
    let paired = context.acting_as(Principal::Device {
        id: device,
        scopes: vec![Scope::Read],
    });
    assert_eq!(
        node.secure
            .service
            .list_host_registrations(&paired, ListHostRegistrations)
            .await
            .unwrap()
            .hosts
            .len(),
        1
    );
}

async fn refused_readers(node: &Node, context: &Connection) {
    let local = node.secure.lifecycle.info().id;
    for principal in [
        Principal::Agent {
            node: local,
            session: SessionId::random(),
            scopes: vec![],
        },
        Principal::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
            scopes: vec![Scope::Read],
        },
    ] {
        let refused = node
            .secure
            .service
            .list_shared_host_registrations(
                &context.acting_as(principal),
                ListSharedHostRegistrations,
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, ErrorCode::Forbidden);
    }
}
