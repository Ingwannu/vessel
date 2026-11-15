//! Real memory-RPC outlets and isolated storage, with no configured node or agent.
use super::*;
use crate::{
    dictation::{endpoint::tests::Server as HttpServer, secret_read::Reader},
    fleet::FleetFiles,
};
use axum::{Router, routing::post};
use std::{num::NonZeroUsize, time::Duration};
use vessel_proto::{
    ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, ProtocolRange, WireError,
    dictation::{DictationChange, DictationProvider, EndpointBase, SecretReference},
};
use vessel_rpc::{ClientConfig, Connection, Server, ServerConfig};

pub(super) const TEST_LIMIT: Duration = Duration::from_secs(60);
const MEMORY_FRAMES: usize = 16;
struct Refuse;
macro_rules! refuse_table {
    ($( $(#[$doc:meta])* $variant:ident / $handler:ident / $name:literal [$scope:ident, $version:tt, $shape:ident]: $request:ty => $response:ty; )*) => {
        impl vessel_proto::Handler for Refuse {
            type Context = ConnectionLife;
            $(async fn $handler(&self, _context: &Self::Context, _request: $request) -> std::result::Result<$response, WireError> {
                Err(WireError::new(ErrorCode::Unsupported, "the outlet fixture serves no calls"))
            })*
        }
    };
}
vessel_proto::method_table!(refuse_table);
fn node() -> NodeInfo {
    NodeInfo {
        id: NodeId::random(),
        name: "test-node".to_owned(),
        hostname: "test-host".to_owned(),
        os: "test".to_owned(),
        arch: "test".to_owned(),
        version: "0".to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: chrono::DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    }
}
pub(super) struct Link {
    pub(super) caller: Caller,
    pub(super) life: ConnectionLife,
    client: Connection,
    stop: CancellationToken,
    task: tokio::task::JoinHandle<
        std::result::Result<(), vessel_rpc::ServeError<vessel_rpc::memory::Disconnected>>,
    >,
}
impl Link {
    pub(super) async fn new() -> Self {
        let life = ConnectionLife::default();
        let context = life.clone();
        let (ready, outlet) = oneshot::channel();
        let (client_end, server_end) =
            vessel_rpc::memory::pair(NonZeroUsize::new(MEMORY_FRAMES).unwrap());
        let stop = CancellationToken::new();
        let server = Server::new(
            Arc::new(Refuse),
            watch::channel(node()).1,
            ServerConfig::default(),
            stop.clone(),
        );
        let task = tokio::spawn(async move {
            server
                .serve(server_end, move |peer| {
                    drop(ready.send(peer.outbound));
                    context
                })
                .await
        });
        let client = Connection::connect(
            client_end,
            ClientInfo {
                name: "dictation-test".to_owned(),
                version: "0".to_owned(),
            },
            &ClientConfig::default(),
        )
        .await
        .unwrap();
        let caller = Caller::new(&life, ConnectionBudget::new(), outlet.await.unwrap());
        Self {
            caller,
            life,
            client,
            stop,
            task,
        }
    }
    pub(super) async fn close(self) {
        self.stop.cancel();
        self.client.closed().await;
        self.task.await.unwrap().unwrap();
    }
    pub(super) fn close_transport(&self) {
        self.stop.cancel();
    }
}
pub(super) struct Fixture {
    pub(super) registry: Registry,
    pub(super) settings: Arc<tokio::sync::RwLock<SetDictation>>,
    pub(super) files: FleetFiles,
    pub(super) name: vessel_proto::SecretName,
    pub(super) data: tempfile::TempDir,
    upstream: HttpServer,
}
impl Fixture {
    pub(super) async fn new() -> Self {
        let upstream = HttpServer::start(Router::new().route(
            "/v1/audio/transcriptions",
            post(|| async { "{\"text\":\"recognized\"}" }),
        ))
        .await;
        let data = tempfile::tempdir().unwrap();
        let files = FleetFiles::new(&vessel_store::DataDir::new(data.path()));
        let name: vessel_proto::SecretName = "TRANSCRIPTION".parse().unwrap();
        crate::fleet::set_secret(&files, &name, "synthetic-recording-key").unwrap();
        let settings = Arc::new(tokio::sync::RwLock::new(endpoint(&name, upstream.base())));
        let registry = Registry::new(
            Factory::new(Reader::new(files.clone())),
            Snapshot::Given(Arc::clone(&settings)),
            &CancellationToken::new(),
        );
        Self {
            registry,
            settings,
            files,
            name,
            data,
            upstream,
        }
    }
    pub(super) async fn finish(self) {
        self.registry.shutdown().await;
        self.upstream.finish().await;
    }
}
pub(super) fn endpoint(name: &vessel_proto::SecretName, base: EndpointBase) -> SetDictation {
    SetDictation {
        provider: Some(DictationChange::Set(DictationProvider::Endpoint)),
        key: Some(DictationChange::Set(SecretReference::from(name.clone()))),
        endpoint_base: Some(DictationChange::Set(base)),
        ..SetDictation::default()
    }
}
pub(super) fn id(name: &str) -> DictationId {
    name.parse().unwrap()
}
pub(super) fn format() -> Pcm16Mono {
    Pcm16Mono::new(vessel_proto::dictation::SampleRate::MIN)
}
pub(super) fn pcm() -> DictationPcm {
    DictationPcm::new(1000_i16.to_le_bytes().to_vec()).unwrap()
}
pub(super) async fn within(test: impl std::future::Future<Output = ()>) {
    vessel_testkit::within(TEST_LIMIT, test).await.unwrap();
}
