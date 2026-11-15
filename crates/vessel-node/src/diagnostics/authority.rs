//! Actual generated guard; direct calls intentionally bypass pending wire admission.
use super::{
    deadline::{self, phase},
    *,
};
use crate::{Config, Node, principal::Principal, service::Connection};
use vessel_harness::HarnessEnv;
use vessel_proto::diagnostics::{
    DiagnosticRefusal, DiagnosticReport, DiagnosticSection, GetDiagnostics,
    MAX_DIAGNOSTIC_COLLECTIONS,
};
use vessel_proto::{ClientInfo, DeviceId, ErrorCode, Handler as _, Scope, SessionId};
use vessel_rpc::{ClientConfig, Peer, Server, ServerConfig};
use vessel_store::DataDir;

#[tokio::test]
async fn generated_guard_checks_scope_before_collector_and_preserves_agent_read() {
    deadline::run(async {
        let (_temp, node) = node().await;
        let transport = transport(&node).await;
        let guard = &node.secure.service;
        let id = node.info().id;
        let mut leases = Vec::new();
        for _ in 0..MAX_DIAGNOSTIC_COLLECTIONS {
            leases.push(guard.0.diagnostics.admission.admit().unwrap());
        }
        for principal in [
            Principal::Device {
                id: DeviceId::random(),
                scopes: vec![Scope::Operate],
            },
            Principal::Agent {
                node: id,
                session: SessionId::random(),
                scopes: vec![Scope::Operate],
            },
        ] {
            let error = phase(
                "no Read before source admission",
                guard.diagnostics(&transport.context(principal), GetDiagnostics {}),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code, ErrorCode::Forbidden);
        }
        for principal in [
            Principal::Owner,
            Principal::Device {
                id: DeviceId::random(),
                scopes: vec![Scope::Read],
            },
            Principal::Agent {
                node: id,
                session: SessionId::random(),
                scopes: vec![Scope::Read],
            },
        ] {
            let error = phase(
                "Read reaches bounded collector",
                guard.diagnostics(&transport.context(principal), GetDiagnostics {}),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code, ErrorCode::Unavailable);
            assert_eq!(
                error.detail,
                Some(vessel_proto::ErrorDetail::Diagnostics {
                    reason: DiagnosticRefusal::Busy
                })
            );
        }
        leases.clear();
        let report = phase(
            "agent Read collects real source report",
            guard.diagnostics(
                &transport.context(Principal::Agent {
                    node: id,
                    session: SessionId::random(),
                    scopes: vec![Scope::Read],
                }),
                GetDiagnostics {},
            ),
        )
        .await
        .unwrap();
        assert_eq!(report.fields().node.id, id);
        assert_recorded_guard_failures(&report);
        let error = phase(
            "self wrapper is not forwardable",
            guard.forward(
                &transport.context(Principal::Owner),
                vessel_proto::Forward {
                    node: id,
                    call: Box::new(vessel_proto::Request::into_call(GetDiagnostics {})),
                },
            ),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        transport.shutdown.cancel();
        drop(transport.client);
        phase("authority transport shuts down", transport.task)
            .await
            .unwrap()
            .unwrap();
        phase("authority node shuts down", node.stop())
            .await
            .unwrap();
    })
    .await;
}

// Snapshot follows the completed calls on the channel and returns newest first.
// This catches an observer moved below the guard's early scope refusal.
fn assert_recorded_guard_failures(report: &DiagnosticReport) {
    let DiagnosticSection::Available { value, .. } = &report.fields().recent_failures else {
        panic!("completed guard failures must be observable")
    };
    let codes: Vec<_> = value
        .entries
        .as_slice()
        .iter()
        .map(|entry| entry.code)
        .collect();
    assert_eq!(
        codes,
        [
            ErrorCode::Unavailable,
            ErrorCode::Unavailable,
            ErrorCode::Unavailable,
            ErrorCode::Forbidden,
            ErrorCode::Forbidden,
        ]
    );
    assert_eq!(value.omitted.get(), 0);
}

pub(super) struct Transport {
    client: vessel_client::NodeClient,
    peer: Peer,
    shutdown: CancellationToken,
    task: tokio::task::JoinHandle<
        Result<(), vessel_rpc::ServeError<vessel_rpc::memory::Disconnected>>,
    >,
}
impl Transport {
    pub(super) fn client(&self) -> &vessel_client::NodeClient {
        &self.client
    }
    pub(super) async fn close(self) {
        self.shutdown.cancel();
        drop(self.client);
        phase("per-node transport shuts down", self.task)
            .await
            .unwrap()
            .unwrap();
    }
    fn context(&self, principal: Principal) -> Connection {
        Connection::new(
            Peer {
                client: self.peer.client.clone(),
                protocol: self.peer.protocol,
                outbound: self.peer.outbound.clone(),
            },
            principal,
        )
    }
}
pub(super) async fn transport(node: &Node) -> Transport {
    let (client_end, server_end) =
        vessel_rpc::memory::pair(std::num::NonZeroUsize::new(4).unwrap());
    let shutdown = CancellationToken::new();
    let server = Server::new(
        Arc::clone(&node.secure.service),
        node.secure.lifecycle.watch_info(),
        ServerConfig::default(),
        shutdown.clone(),
    );
    let (sender, captured) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        server
            .serve(server_end, |peer| {
                let copy = Peer {
                    client: peer.client.clone(),
                    protocol: peer.protocol,
                    outbound: peer.outbound.clone(),
                };
                sender.send(copy).unwrap();
                Connection::new(peer, Principal::Owner)
            })
            .await
    });
    let client = phase(
        "admit authority fixture",
        vessel_client::NodeClient::over(
            client_end,
            ClientInfo {
                name: "diagnostic-authority".to_owned(),
                version: "0".to_owned(),
            },
            &ClientConfig::default(),
        ),
    )
    .await
    .unwrap();
    let peer = phase("capture admitted peer", captured).await.unwrap();
    Transport {
        client,
        peer,
        shutdown,
        task,
    }
}

async fn node() -> (tempfile::TempDir, Node) {
    let temp = tempfile::tempdir().unwrap();
    let mut config = Config::new(
        DataDir::new(temp.path().join("data")),
        HarnessEnv::new(temp.path().join("user")),
    );
    config.bind = None;
    config.harness_lookup = Lookup::new(Vec::<std::path::PathBuf>::new());
    let node = phase("start authority fixture", Node::start(config))
        .await
        .unwrap();
    (temp, node)
}
