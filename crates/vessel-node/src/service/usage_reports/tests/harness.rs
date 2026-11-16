//! Real NodeService/Guarded/actor with a real framed handshake, without assigning a pending RPC.
use super::*;
use crate::limits::test_support::PhaseWait;
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio_util::sync::CancellationToken;
use vessel_harness::{
    AccountSourceAccess, AccountUsageProbe, HarnessEnv, Lookup, RecordedUsageFormat,
};
use vessel_proto::{ClientInfo, DeviceId, Scope, usage_reports::BuiltinUsageSource};
use vessel_rpc::{Framed, Server, ServerConfig};
const FRAME_BYTES: usize = 65_536;
pub(super) const SERVICE_TEST_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);
pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    node: Option<crate::Node>,
    pub(super) owner: Connection,
    pub(super) reader: Connection,
    pub(super) reads: Arc<AtomicUsize>,
    _client: vessel_client::NodeClient,
    server: Option<tokio::task::JoinHandle<()>>,
    stop: CancellationToken,
    _cancel: tokio_util::sync::DropGuard,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.cancel();
        if let Some(server) = &self.server {
            server.abort();
        }
        if let Some(node) = &self.node {
            node.secure.lifecycle.stop(vessel_proto::StopWhen::Now);
        }
    }
}
impl Fixture {
    pub(super) async fn start() -> Self {
        Self::with_access(|home, seen| {
            AccountSourceAccess::new(home, move |name| {
                let _previous = seen.fetch_add(1, Ordering::SeqCst);
                (name == "CURSOR_ACCESS_TOKEN").then(|| "synthetic-service-source".into())
            })
        })
        .phase("usage phase: Self::with_access")
        .await
    }
    pub(super) async fn with_access(
        access: impl FnOnce(std::path::PathBuf, Arc<AtomicUsize>) -> AccountSourceAccess,
    ) -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let body = root.path().join("report.json");
        std::fs::write(&body,br#"{"plan":"Fixture","windows":[],"balances":[{"id":"credits","label":"Credits","used":3,"remaining":7,"limit":10,"unit":"credits","resets_at":null,"tone":null}],"details":[{"id":"private","label":"Private","value":"synthetic-only","access":"operate","tone":null}]}"#).unwrap();
        let reads = Arc::new(AtomicUsize::new(0));
        let mut config = crate::Config::new(
            vessel_store::DataDir::new(root.path().join("data")),
            HarnessEnv::new(home.clone()),
        );
        config.bind = None; // This test uses only the explicit in-memory carrier.
        config.harness_lookup = Lookup::new([]);
        config.usage_sources = AccountUsageProbe::Recorded {
            root: root.path().to_owned(),
            access: access(home, Arc::clone(&reads)),
            source: BuiltinUsageSource::Cursor,
            body,
            status: 200,
            retry_after: None,
            format: RecordedUsageFormat::Report,
        };
        let node = crate::Node::start(config)
            .phase("usage phase: crate::Node::start")
            .await
            .unwrap();
        let stop = CancellationToken::new();
        let cancel = stop.clone().drop_guard();
        let server = Server::new(
            Arc::clone(&node.secure.service),
            node.secure.lifecycle.watch_info(),
            ServerConfig::default(),
            stop.clone(),
        );
        let (local, remote) = tokio::io::duplex(FRAME_BYTES);
        let (reader, writer) = tokio::io::split(remote);
        let (captured, connection) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let _closed = server
                .serve(
                    Framed::new(reader, writer, NonZeroUsize::new(FRAME_BYTES).unwrap()),
                    move |peer| {
                        let owner = Connection::owner(peer);
                        assert!(
                            captured
                                .send(owner.acting_as(crate::principal::Principal::Owner))
                                .is_ok()
                        );
                        owner
                    },
                )
                .await;
        });
        let (reader, writer) = tokio::io::split(local);
        let client = vessel_client::NodeClient::over(
            Framed::new(reader, writer, NonZeroUsize::new(FRAME_BYTES).unwrap()),
            ClientInfo {
                name: "source-service-test".into(),
                version: "0".into(),
            },
            &vessel_client::ClientConfig::default(),
        )
        .phase("usage phase: vessel_client::NodeClient::over")
        .await
        .unwrap();
        let owner = connection.phase("usage phase: connection").await.unwrap();
        let reader = owner.acting_as(crate::principal::Principal::Device {
            id: DeviceId::random(),
            scopes: vec![Scope::Read],
        });
        Self {
            root,
            node: Some(node),
            owner,
            reader,
            reads,
            _client: client,
            server: Some(server),
            stop,
            _cancel: cancel,
        }
    }
    pub(super) fn service(&self) -> &crate::guard::Guarded {
        &self.node.as_ref().unwrap().secure.service
    }
    pub(super) fn handler(&self) -> Arc<crate::guard::Guarded> {
        Arc::clone(&self.node.as_ref().unwrap().secure.service)
    }
    pub(super) async fn finish(mut self) {
        self.stop.cancel();
        self.node
            .take()
            .unwrap()
            .stop()
            .phase("usage phase: self.node.take.unwrap.stop")
            .await
            .unwrap();
        self.server
            .take()
            .unwrap()
            .phase("framed server drain")
            .await
            .unwrap();
    }
    pub(super) fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
    pub(super) fn source_request(policy: ExternalLookup) -> SetUsageSourceLookup {
        SetUsageSourceLookup {
            source: "cursor".parse().unwrap(),
            policy,
            disclosure_version: vessel_proto::usage_reports::UsageDisclosureVersion::INITIAL,
        }
    }
    pub(super) async fn list(&self, who: &Connection, refresh: LimitRefresh) -> UsageReports {
        self.service()
            .list_usage_reports(
                who,
                ListUsageReports {
                    scope: UsageReportScope::Machine,
                    refresh,
                },
            )
            .phase("usage phase: self.service .list_usage_reports")
            .await
            .unwrap()
    }
    pub(super) async fn allow(&self) {
        let _policy = self
            .service()
            .set_usage_lookup(
                &self.owner,
                vessel_proto::limits::SetUsageLookup {
                    policy: ExternalLookup::On,
                },
            )
            .phase("usage phase: self .service .set_usage_lookup")
            .await
            .unwrap();
        let _source = self
            .service()
            .set_usage_source_lookup(&self.owner, Self::source_request(ExternalLookup::On))
            .phase("usage phase: self .service .set_usage_source_lookup")
            .await
            .unwrap();
    }
}
