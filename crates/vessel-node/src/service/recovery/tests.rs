//! Actual RPC cancellation and final admission with the real fake ACP process.
use super::*;
use crate::{Config, Node};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpListener;
use vessel_client::NodeClient;
use vessel_harness::fakes::{Role, install};
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::automation::PromptText;
use vessel_proto::restart::SetRestartRecovery;
use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
use vessel_proto::{
    AgentEvent, ClientInfo, ContentBlock, Delivery, EventSeq, HarnessKind, SessionMode, SessionSpec,
};
use vessel_store::DataDir;
use vessel_testkit::Fake;

struct Fixture {
    _temp: TempDir,
    data: DataDir,
    home: std::path::PathBuf,
    bin: std::path::PathBuf,
    work: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = DataDir::new(temp.path().join("data"));
        let home = temp.path().join("home");
        let bin = temp.path().join("bin");
        let work = temp.path().join("work");
        for path in [data.root(), home.as_path(), bin.as_path(), work.as_path()] {
            std::fs::create_dir_all(path).unwrap();
        }
        drop(
            install(
                &bin,
                HarnessKind::ClaudeCode,
                Role::Terminal,
                &Fake::Tui.path().unwrap(),
            )
            .unwrap(),
        );
        drop(
            install(
                &bin,
                HarnessKind::ClaudeCode,
                Role::Structured,
                &Fake::Acp.path().unwrap(),
            )
            .unwrap(),
        );
        let f = Self {
            _temp: temp,
            data,
            home,
            bin,
            work,
        };
        f.profile(None);
        f
    }
    fn profile(&self, gate: Option<std::net::SocketAddr>) {
        let env = gate
            .map(|address| format!("env = {{ FAKE_ACP_LOAD_GATE = \"{address}\" }}\n"))
            .unwrap_or_default();
        std::fs::write(
            self.data.profiles_path(),
            format!("[profiles.recovery]\nharness = \"claude\"\n{env}"),
        )
        .unwrap();
    }
    async fn start(&self) -> Node {
        let mut config = Config::new(self.data.clone(), HarnessEnv::new(self.home.clone()));
        config.bind = Some("127.0.0.1:0".parse().unwrap());
        config.harness_lookup = Lookup::new([self.bin.clone()]);
        Node::start(config).await.unwrap()
    }
    async fn client(&self, node: &Node) -> NodeClient {
        let token = self.data.read_token().unwrap().expose().parse().unwrap();
        NodeClient::connect(
            &node.address().unwrap().to_string().parse().unwrap(),
            &token,
            ClientInfo {
                name: "recovery-test".into(),
                version: "0".into(),
            },
        )
        .await
        .unwrap()
    }
    async fn active(&self, client: &NodeClient) -> vessel_proto::SessionInfo {
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
        .with_cwd(self.work.clone())
        .with_profile("recovery".parse().unwrap());
        let id = client.create_session(spec).await.unwrap().id;
        let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
        loop {
            let event = stream.next().await.unwrap().unwrap();
            if matches!(
                event.event,
                SessionEvent::Status {
                    status: SessionStatus::Running
                }
            ) {
                break;
            }
        }
        let delivered = client
            .prompt(
                id,
                vec![ContentBlock::Text {
                    text: "wait".into(),
                }],
                Delivery::Start,
            )
            .await
            .unwrap();
        assert_eq!(delivered, vessel_proto::Delivered::Started);
        client.session(id).await.unwrap()
    }
}
fn change(enabled: bool) -> SetNodeSettings {
    SetNodeSettings {
        restart_recovery: Some(SetRestartRecovery {
            enabled: Some(enabled),
            prompt: Some("loaded".parse::<PromptText>().unwrap()),
        }),
        ..SetNodeSettings::default()
    }
}
async fn set(client: &NodeClient, enabled: bool) {
    assert_eq!(
        client
            .call(change(enabled))
            .await
            .unwrap()
            .restart_recovery
            .enabled,
        enabled
    );
}
async fn done(node: &Node) {
    let mut done = node.secure.service.0.recovery.boot_done();
    let _ready = tokio::time::timeout(Duration::from_secs(20), done.wait_for(|done| *done))
        .await
        .unwrap()
        .unwrap();
}
#[derive(Clone, Copy)]
enum Invalidation {
    Off,
    OffThenOn,
    Stop,
}
async fn final_admission(invalidation: Invalidation) {
    vessel_testkit::within(
        Duration::from_secs(60),
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&owner, true).await;
            let original = f.active(&owner).await;
            node.stop().await.unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            f.profile(Some(listener.local_addr().unwrap()));
            let node = f.start().await;
            let owner = f.client(&node).await;
            let (mut load, _) = listener.accept().await.unwrap();
            let length = load.read_u32().await.unwrap();
            assert!(length <= 4096);
            let mut bytes = vec![0; usize::try_from(length).unwrap()];
            assert_eq!(load.read_exact(&mut bytes).await.unwrap(), bytes.len());
            assert_eq!(
                String::from_utf8(bytes).unwrap(),
                original.harness_session.unwrap().as_str()
            );
            let control = &node.secure.service.0.recovery;
            let (entered, release) = control.pause_admission();
            let cancelled = control.attempt_cancelled(original.id);
            load.write_all(&[1]).await.unwrap();
            drop(load);
            tokio::time::timeout(Duration::from_secs(20), entered)
                .await
                .unwrap()
                .unwrap();
            let stopping = match invalidation {
                Invalidation::Off => {
                    set(&owner, false).await;
                    None
                }
                Invalidation::OffThenOn => {
                    set(&owner, false).await;
                    set(&owner, true).await;
                    None
                }
                Invalidation::Stop => {
                    let owner = owner.clone();
                    let id = original.id;
                    let stopping = tokio::spawn(async move { owner.kill(id).await });
                    tokio::time::timeout(Duration::from_secs(20), cancelled.cancelled())
                        .await
                        .unwrap();
                    Some(stopping)
                }
            };
            release.send(()).unwrap();
            if let Some(stopping) = stopping {
                stopping.await.unwrap().unwrap();
            }
            done(&node).await;
            let records = owner
                .read(original.id, EventSeq::ZERO, 128)
                .await
                .unwrap()
                .records;
            let continuation = vec![ContentBlock::Text {
                text: "loaded".into(),
            }];
            assert!(!records.iter().any(|record| {
                matches!(&record.event, SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { content },
                } if content == &continuation)
            }));
            assert!(!owner.session(original.id).await.unwrap().status.is_live());
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn off_wins_at_the_final_actual_prompt_admission() {
    final_admission(Invalidation::Off).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn off_then_on_cannot_revive_the_held_final_prompt_admission() {
    final_admission(Invalidation::OffThenOn).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn person_stop_wins_at_the_final_actual_prompt_admission() {
    final_admission(Invalidation::Stop).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_cancel_during_the_actual_settings_write_does_not_cancel_saved_off_apply() {
    vessel_testkit::within(
        Duration::from_secs(60),
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&owner, true).await;
            let settings = &node.secure.service.0.settings;
            let (entered, release) = settings.pause_write();
            let caller = {
                let owner = owner.clone();
                tokio::spawn(async move { owner.call(change(false)).await })
            };
            entered.await.unwrap();
            caller.abort();
            assert!(caller.await.unwrap_err().is_cancelled());
            let _barrier = owner.call(vessel_proto::GetNode).await.unwrap();
            release.send(()).unwrap();
            let effective = owner
                .call(GetNodeSettings {
                    include_restart_recovery: true,
                })
                .await
                .unwrap();
            assert!(!effective.restart_recovery.enabled);
            assert!(
                !crate::settings::read_saved(&f.data)
                    .restart_recovery
                    .unwrap()
                    .effective()
                    .enabled
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                assert_eq!(
                    std::fs::metadata(f.data.settings_path())
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777,
                    0o600
                );
            }
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_cancel_after_saved_off_keeps_cleanup_ahead_of_the_next_on() {
    vessel_testkit::within(
        Duration::from_secs(60),
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&owner, true).await;
            let settings = &node.secure.service.0.settings;
            let (_, generation) = settings.policy().snapshot();
            let (entered, release) = settings.pause_cleanup();
            let caller = {
                let owner = owner.clone();
                tokio::spawn(async move { owner.call(change(false)).await })
            };
            entered.await.unwrap();
            assert!(
                !crate::settings::read_saved(&f.data)
                    .restart_recovery
                    .unwrap()
                    .effective()
                    .enabled
            );
            assert!(!settings.policy().snapshot().0.enabled);
            caller.abort();
            assert!(caller.await.unwrap_err().is_cancelled());
            let _barrier = owner.call(vessel_proto::GetNode).await.unwrap();
            let next = {
                let owner = owner.clone();
                tokio::spawn(async move { owner.call(change(true)).await })
            };
            release.send(()).unwrap();
            assert!(next.await.unwrap().unwrap().restart_recovery.enabled);
            assert!(
                settings.policy().check(generation).is_err(),
                "On cannot resurrect the invalidated boot generation"
            );
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}

mod failures;

mod eligibility;
