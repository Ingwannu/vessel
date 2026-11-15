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

pub(in crate::dictation) const TEST_LIMIT: Duration = Duration::from_secs(60);
const MEMORY_FRAMES: usize = 16;
struct Refuse;
macro_rules! refuse_table {
    ($( $(#[$doc:meta])* $variant:ident / $handler:ident / $name:literal [$scope:ident, $version:tt, $shape:ident]: $request:ty => $response:ty; )*) => {
        impl vessel_proto::Handler for Refuse {
            type Context = Arc<crate::service::Connection>;
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
/// Assembled-service tests share the actual connection factory and close path.
pub(crate) struct Link {
    pub(in crate::dictation) caller: Caller,
    /// Retained actual context is a positive control for physical outlet closure.
    pub(crate) context: Arc<crate::service::Connection>,
    client: Connection,
    stop: CancellationToken,
    task: tokio::task::JoinHandle<
        std::result::Result<(), vessel_rpc::ServeError<vessel_rpc::memory::Disconnected>>,
    >,
}
impl Link {
    pub(crate) async fn new() -> Self {
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
                    let context = Arc::new(crate::service::Connection::owner(peer));
                    drop(ready.send(Arc::clone(&context)));
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
        let context = outlet.await.unwrap();
        let caller = context.dictation_context().unwrap().caller().clone();
        Self {
            caller,
            context,
            client,
            stop,
            task,
        }
    }
    pub(crate) async fn close(self) {
        self.stop.cancel();
        self.client.closed().await;
        self.task.await.unwrap().unwrap();
    }
    pub(in crate::dictation) fn close_transport(&self) {
        self.stop.cancel();
    }
}
pub(in crate::dictation) struct Fixture {
    pub(in crate::dictation) registry: Registry,
    pub(in crate::dictation) settings: Arc<tokio::sync::RwLock<SetDictation>>,
    pub(in crate::dictation) files: FleetFiles,
    pub(in crate::dictation) name: vessel_proto::SecretName,
    pub(in crate::dictation) data: tempfile::TempDir,
    upstream: HttpServer,
}
impl Fixture {
    pub(in crate::dictation) async fn new() -> Self {
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
    pub(in crate::dictation) async fn finish(self) {
        self.registry.shutdown().await;
        self.upstream.finish().await;
    }
}
pub(in crate::dictation) fn endpoint(
    name: &vessel_proto::SecretName,
    base: EndpointBase,
) -> SetDictation {
    SetDictation {
        provider: Some(DictationChange::Set(DictationProvider::Endpoint)),
        key: Some(DictationChange::Set(SecretReference::from(name.clone()))),
        endpoint_base: Some(DictationChange::Set(base)),
        ..SetDictation::default()
    }
}
pub(in crate::dictation) fn id(name: &str) -> DictationId {
    name.parse().unwrap()
}
pub(in crate::dictation) fn format() -> Pcm16Mono {
    Pcm16Mono::new(vessel_proto::dictation::SampleRate::MIN)
}
pub(in crate::dictation) fn pcm() -> DictationPcm {
    DictationPcm::new(1000_i16.to_le_bytes().to_vec()).unwrap()
}
pub(in crate::dictation) async fn within(test: impl Future<Output = ()>) {
    vessel_testkit::within(TEST_LIMIT, test).await.unwrap();
}
