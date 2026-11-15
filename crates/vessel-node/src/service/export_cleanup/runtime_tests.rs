//! Real callback transport and holder checks; metadata assignment is still pending.
//! Returned manifests are synthetic, because a current source refuses them early.

use crate::fleet::{self, DEFAULT_GRANT, FleetFiles, FleetInit, FleetTargets};
use crate::metadata_export::tests::{exported, info};
use crate::principal::Principal;
use crate::transfers::{Holder, TransferError};
use crate::{Config, Node};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use vessel_client::secure::{SecureConnector, WsDialer};
use vessel_client::{Connector as _, DeviceCredential, SecureTarget};
use vessel_proto::{
    ClientInfo, CloseTransfer, ErrorCode, InviteDevice, NodeId, OriginBy, PairingOffer, Reply,
    Request as _, RouteUrl, Scope, SessionListing,
};
use vessel_relay::{LinkState, RelayConfig};
use vessel_store::DataDir;

const CALLBACK_TEST_BUDGET: Duration = Duration::from_secs(60);
const INVITE_TTL_SECS: u32 = 600;
const RELAY_TOKEN: &str = "export-cleanup-fixture";

fn config(root: &Path) -> Config {
    let mut config = Config::new(
        DataDir::new(root.join("data")),
        vessel_harness::HarnessEnv::new(root.join("user")),
    );
    config.bind = None;
    config.harness_lookup = vessel_harness::Lookup::new(Vec::<PathBuf>::new());
    config.fleet.targets = FleetTargets::Only(Vec::new());
    config
}

pub(super) struct CallbackFleet {
    _home_root: tempfile::TempDir,
    _member_root: tempfile::TempDir,
    home: Node,
    member: Node,
    relay_stop: CancellationToken,
    relay: tokio::task::JoinHandle<()>,
}

impl CallbackFleet {
    /// Reuse the authenticated callback fixture for envelope wiring tests.
    pub(super) async fn start() -> Self {
        let (relay_url, relay_stop, relay) = start_relay().await;
        let home_root = tempfile::tempdir().unwrap();
        let mut home_config = config(home_root.path());
        home_config.name = Some("cleanup-home".into());
        let instructions = home_root.path().join("instructions.md");
        std::fs::write(&instructions, "fixture instructions").unwrap();
        let _created = fleet::init(
            &FleetFiles::new(&home_config.data_dir),
            &FleetInit {
                instructions: Some(instructions),
                skills: None,
                skip: Vec::new(),
            },
        )
        .unwrap();
        let token_file = home_root.path().join("relay-token");
        std::fs::write(&token_file, RELAY_TOKEN).unwrap();
        home_config.relays = vec![relay_url];
        home_config.relay_token_file = Some(token_file);
        let home = Node::start(home_config).await.unwrap();
        for (_, mut state) in home.relay_links() {
            let _registered = state
                .wait_for(|state| *state == LinkState::Registered)
                .await
                .unwrap();
        }
        let offer = home
            .secure
            .devices
            .invite(&InviteDevice {
                label: Some("cleanup-member".into()),
                ttl_secs: INVITE_TTL_SECS,
                scopes: Some(vec![Scope::Sync, Scope::Relay]),
            })
            .unwrap()
            .link
            .decode()
            .unwrap();
        let member_root = tempfile::tempdir().unwrap();
        let member = callback_member(member_root.path(), offer, home.info().id).await;
        Self {
            _home_root: home_root,
            _member_root: member_root,
            home,
            member,
            relay_stop,
            relay,
        }
    }

    /// Expose the fixture node so sibling tests can drive its guarded API.
    pub(super) fn home(&self) -> &Node {
        &self.home
    }

    /// Expose the joined node for the real member-to-home relay path.
    pub(super) fn member(&self) -> &Node {
        &self.member
    }

    /// Use only this temporary node's own token at the local socket.
    pub(super) async fn owner(&self, node: &Node) -> vessel_client::NodeClient {
        let root = if node.info().id == self.home.info().id {
            self._home_root.path()
        } else {
            assert_eq!(node.info().id, self.member.info().id);
            self._member_root.path()
        };
        let token = DataDir::new(root.join("data")).read_token().unwrap();
        let credential = vessel_client::LocalCredential::Token(token.expose().parse().unwrap());
        vessel_client::NodeClient::connect_local(
            node.local_endpoint(),
            &credential,
            ClientInfo {
                name: "cleanup-local-fixture".into(),
                version: "0".into(),
            },
        )
        .await
        .unwrap()
    }

    /// Settle the temporary nodes and relay without touching any live service.
    pub(super) async fn stop(self) {
        self.member.stop().await.unwrap();
        self.home.stop().await.unwrap();
        self.relay_stop.cancel();
        self.relay.await.unwrap();
    }
}

async fn start_relay() -> (RouteUrl, CancellationToken, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_url = format!("ws://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let relay_stop = CancellationToken::new();
    let stopped = relay_stop.clone();
    let relay = tokio::spawn(async move {
        vessel_relay::serve(
            listener,
            RelayConfig::new(vessel_proto::RelayToken::new(RELAY_TOKEN).unwrap()),
            stopped,
        )
        .await
        .unwrap();
    });
    (relay_url, relay_stop, relay)
}

async fn callback_member(root: &Path, offer: PairingOffer, home: NodeId) -> Node {
    let mut member_config = config(root);
    member_config.name = Some("cleanup-member".into());
    let files = FleetFiles::new(&member_config.data_dir);
    let member = Node::start(member_config).await.unwrap();
    // Own the membership connection explicitly in this fixture. Cancelling
    // the existing epoch before joining prevents a second automatic grant
    // producer; no timing wait or test-only product control is needed.
    member.secure.service.0.fleet.membership_epoch().cancel();
    let _joined = fleet::join(&files, &offer, "cleanup-member", DEFAULT_GRANT.to_vec())
        .await
        .unwrap();
    // Completing the ordinary authenticated grant is the callback barrier.
    let target = SecureTarget {
        key: offer.key,
        relays: offer.relays,
        direct: offer.direct,
    };
    let dialer = WsDialer::for_routes(target.routes()).unwrap();
    let credential = DeviceCredential::read(&files.credential()).unwrap();
    let connector = SecureConnector::new(dialer, target, home, credential).unwrap();
    let client = connector
        .connect(ClientInfo {
            name: "cleanup-fixture".into(),
            version: "0".into(),
        })
        .await
        .unwrap();
    let membership = serde_json::from_slice(&std::fs::read(files.membership()).unwrap()).unwrap();
    member
        .secure
        .service
        .0
        .links
        .grant(&client, &membership, &CancellationToken::new())
        .await
        .unwrap();
    member.secure.service.0.links.connected_home(client);
    assert!(
        !member.relay_links().is_empty(),
        "no own routes: delivery must use callback"
    );
    member
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn metadata_refusal_closes_the_exact_holder_through_forward_relay_and_callback() {
    vessel_testkit::within(CALLBACK_TEST_BUDGET, async {
        let fleet = CallbackFleet::start().await;
        let source = &fleet.member.secure.service.0;
        let home_id = fleet.home.info().id;
        let target = fleet.member.info().id;
        let delivering_device = source.links.home_device(home_id).await.unwrap();
        for forwarding in [&fleet.home.secure.service, &fleet.member.secure.service] {
            // A member-to-itself route intentionally traverses its home: this
            // exercises Relay and callback Deliver without claiming three nodes.
            let origin_node = forwarding.0.links.node();
            let owner = Holder::Delegated {
                device: delivering_device,
                node: origin_node,
                by: OriginBy::Owner,
            };
            let staged = source.transfers.stage(false).unwrap();
            let transfer = source
                .transfers
                .commit(staged, owner, std::iter::empty())
                .unwrap();
            assert!(
                matches!(source.transfers.close(Holder::Owner, transfer).await,
                Err(TransferError::Unknown(id)) if id == transfer)
            );
            let mut current = info();
            current.listing = SessionListing::Background;
            let mut value = exported(current);
            let Reply::ExportSession(manifest) = value.reply.as_mut() else {
                panic!("expected export");
            };
            manifest.transfer = transfer;
            let via = Arc::clone(forwarding);
            let result = forwarding
                .0
                .settle_export(
                    &value,
                    vessel_proto::PROTOCOL_VERSION,
                    move |id| async move {
                        assert_eq!(id, transfer);
                        let response = via
                            .0
                            .pass_on(
                                &Principal::Owner,
                                target,
                                Box::new(CloseTransfer { transfer: id }.into_call()),
                            )
                            .await?;
                        let _closed = CloseTransfer::response_from(*response.reply).unwrap();
                        Ok(())
                    },
                )
                .await;
            let error = result.unwrap_err();
            assert_eq!(error.code, ErrorCode::Unsupported);
            assert!(
                matches!(source.transfers.close(owner, transfer).await,
                Err(TransferError::Unknown(id)) if id == transfer),
                "callback close did not settle"
            );
        }
        fleet.stop().await;
    })
    .await
    .unwrap();
}
