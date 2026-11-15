//! Real node, fake ACP, and a context captured by an actual RPC handshake.
use crate::{
    Config, Node, guard::Guarded, principal::Principal, service::Connection, sessions::Found,
};
use std::{future::Future, num::NonZeroUsize, sync::Arc, time::Duration};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use vessel_harness::{
    HarnessEnv, Lookup,
    fakes::{Role, install},
};
use vessel_proto::{
    ClientInfo, HarnessKind, SessionId, SessionMode, SessionSpec, SessionStatus, StopWhen,
};
use vessel_rpc::{ClientConfig, Server, ServerConfig, memory};

pub(super) const SAFETY: Duration = Duration::from_secs(60);
pub(super) async fn phase<T>(name: &str, job: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(15), job)
        .await
        .unwrap_or_else(|_| panic!("launch phase timed out: {name}"))
}
pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) node: Option<Node>,
}
impl Fixture {
    pub(super) async fn new() -> Self {
        Self::configured(None).await
    }
    pub(super) async fn configured(profiles: Option<&str>) -> Self {
        let root = tempfile::tempdir().unwrap();
        for sub in ["bin", "home", "work", "data"] {
            std::fs::create_dir(root.path().join(sub)).unwrap();
        }
        let data = vessel_store::DataDir::new(root.path().join("data"));
        if let Some(profiles) = profiles {
            // Use the same private atomic writer as operator-owned profiles;
            // fixture setup must not depend on the runner's creation mask.
            data.write_profiles(profiles).unwrap();
            assert!(!data.read_profiles().unwrap().unwrap().readable_by_others);
        }
        let bin = root.path().join("bin");
        let _installed = install(
            &bin,
            HarnessKind::ClaudeCode,
            Role::Structured,
            &vessel_testkit::Fake::Acp.path().unwrap(),
        )
        .unwrap();
        let _installed = install(
            &bin,
            HarnessKind::ClaudeCode,
            Role::Terminal,
            &vessel_testkit::Fake::Tui.path().unwrap(),
        )
        .unwrap();
        let mut config = Config::new(data, HarnessEnv::new(root.path().join("home")));
        config.bind = None;
        config.harness_lookup = Lookup::new([bin]);
        let node = phase("node starts", Node::start(config)).await.unwrap();
        Self {
            root,
            node: Some(node),
        }
    }
    pub(super) fn service(&self) -> Arc<Guarded> {
        Arc::clone(&self.node.as_ref().unwrap().secure.service)
    }
    pub(super) async fn context(&self, principal: Principal) -> Context {
        let stop = CancellationToken::new();
        let (_info, watch) = tokio::sync::watch::channel(self.node.as_ref().unwrap().info());
        let server = Server::new(self.service(), watch, ServerConfig::default(), stop.clone());
        let (client, transport) = memory::pair(NonZeroUsize::new(16).unwrap());
        let (send, receive) = oneshot::channel();
        let task = tokio::spawn(async move {
            server
                .serve(transport, move |peer| {
                    let context = Connection::new(peer, principal.clone());
                    send.send(context.acting_as(principal)).unwrap();
                    context
                })
                .await
                .unwrap();
        });
        let rpc = phase(
            "context handshake",
            vessel_rpc::Connection::connect(
                client,
                ClientInfo {
                    name: "launch-profile-test".into(),
                    version: "0".into(),
                },
                &ClientConfig::default(),
            ),
        )
        .await
        .unwrap();
        let connection = phase("captured context", receive).await.unwrap();
        Context {
            rpc,
            connection,
            stop,
            task: Some(task),
        }
    }
    pub(super) async fn delegated_agent(&self, origin_node: vessel_proto::NodeId) -> Context {
        self.context(Principal::delegated(
            vessel_proto::DeviceId::random(),
            &vessel_proto::Scope::ALL,
            vessel_proto::Origin {
                node: origin_node,
                by: vessel_proto::OriginBy::Agent {
                    session: SessionId::random(),
                    depth: 0,
                },
                scopes: vessel_proto::Scope::ALL.to_vec(),
            },
            vessel_proto::NodeId::random(),
        ))
        .await
    }
    pub(super) async fn running(&self, context: &Connection) -> vessel_session::SessionHandle {
        use vessel_proto::Handler as _;
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
        .with_cwd(self.root.path().join("work"));
        let service = self.service();
        let info = phase(
            "create structured session",
            service.create_session(context, vessel_proto::CreateSession { spec }),
        )
        .await
        .unwrap();
        let Found::Live { handle, .. } = phase("created handle", service.0.sessions.find(info.id))
            .await
            .unwrap()
        else {
            panic!("not live")
        };
        let mut watch = handle.watch_info();
        phase("fake ACP running", async {
            loop {
                let status = watch.borrow_and_update().status.clone();
                if status == SessionStatus::Running {
                    break;
                }
                assert!(status.is_live(), "agent failed: {status:?}");
                watch.changed().await.unwrap();
            }
        })
        .await;
        handle
    }
    pub(super) async fn finish(mut self) {
        phase("node stops and drains", self.node.take().unwrap().stop())
            .await
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(node) = &self.node {
            node.secure.service.0.lifecycle.stop(StopWhen::Now);
        }
    }
}
pub(super) struct Context {
    pub(super) rpc: vessel_rpc::Connection,
    pub(super) connection: Connection,
    stop: CancellationToken,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Context {
    pub(super) async fn finish(mut self) {
        self.stop.cancel();
        phase("context server ends", self.task.take().unwrap())
            .await
            .unwrap();
        phase("context driver ends", self.rpc.closed()).await;
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
pub(super) fn selected(id: &str, value: &str) -> vessel_proto::launch_profiles::LaunchChoice {
    vessel_proto::launch_profiles::LaunchChoice::try_from(vessel_proto::OptionChoice {
        id: vessel_proto::OptionId::new(id),
        value: vessel_proto::ChoiceValue::Select {
            value: vessel_proto::OptionValueId::new(value),
        },
    })
    .unwrap()
}
pub(super) fn bundle(ids: &[(&str, &str)]) -> vessel_proto::launch_profiles::LaunchChoices {
    ids.iter()
        .map(|(id, value)| selected(id, value))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}
pub(super) fn unrelated_agent() -> Principal {
    Principal::Agent {
        node: vessel_proto::NodeId::random(),
        session: SessionId::random(),
        scopes: vessel_proto::Scope::ALL.to_vec(),
    }
}
