//! Actual provider admission and owned settings cancellation while wire assignment is pending.
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
use vessel_proto::settings::SetNodeSettings;
use vessel_proto::{
    AgentEvent, ClientInfo, ContentBlock, Delivery, EventSeq, HarnessKind, SessionMode, SessionSpec,
};
use vessel_store::DataDir;
use vessel_testkit::Fake;

/// Test-only safety nets; these do not change product admission deadlines.
const EVENT_PAGE: u32 = 128;
const WAIT: Duration = Duration::from_secs(20);
const CASE_DEADLINE: Duration = Duration::from_secs(60);

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
        // Invitations require an advertised route. The pairing fixture
        // replaces this sentinel with the actual ephemeral listener before dial.
        config.public = vec!["ws://127.0.0.1:1".parse().unwrap()];
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
    async fn active(&self, node: &Node, client: &NodeClient) -> vessel_proto::SessionInfo {
        let (id, through) = self.start_turn(client).await;
        let info = committed(node, id, through).await;
        let kept = client.read(id, EventSeq::ZERO, EVENT_PAGE).await.unwrap();
        assert!(kept.records.iter().any(|record| matches!(
            record.event,
            SessionEvent::Agent {
                event: AgentEvent::TurnStarted { .. }
            }
        )));
        info
    }
    async fn start_turn(&self, client: &NodeClient) -> (SessionId, EventSeq) {
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
        let through = loop {
            let record = stream.next().await.unwrap().unwrap();
            if matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { .. }
                }
            ) {
                break record.seq.next().unwrap();
            }
        };
        (id, through)
    }
}
async fn committed(
    node: &Node,
    session: SessionId,
    through: EventSeq,
) -> vessel_proto::SessionInfo {
    // A live event precedes its info-watch update. Wait on the real writer's
    // acknowledged prefix, so a kill test cannot race its own setup commit.
    let crate::sessions::Found::Live { mut durability, .. } =
        node.secure.service.0.sessions.find(session).await.unwrap()
    else {
        panic!("the active fake turn ended before its commit barrier");
    };
    crate::persist::wait_until(session, &mut durability, through)
        .await
        .unwrap();
    let info = node
        .secure
        .service
        .0
        .sessions
        .store()
        .session(session)
        .await
        .unwrap()
        .unwrap();
    assert!(info.next_seq >= through);
    info
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
async fn set(node: &Node, enabled: bool) {
    assert_eq!(
        node.secure
            .service
            .0
            .settings
            .change(change(enabled).into())
            .await
            .unwrap()
            .restart_recovery
            .enabled,
        enabled
    );
}
async fn done(node: &Node) {
    let mut done = node.secure.service.0.recovery.boot_done();
    let _ready = tokio::time::timeout(WAIT, done.wait_for(|done| *done))
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
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&node, true).await;
            let original = f.active(&node, &owner).await;
            node.stop().await.unwrap();
            let gate = LoadGate::install(&f).await;
            let node = f.start().await;
            let owner = f.client(&node).await;
            let (loaded, mut load) = gate.arrive().await;
            assert_eq!(loaded, original.harness_session.unwrap().as_str());
            let control = &node.secure.service.0.recovery;
            let (entered, release) = control.pause_admission();
            let cancelled = control.attempt_cancelled(original.id);
            load.write_all(&[1]).await.unwrap();
            drop(load);
            tokio::time::timeout(WAIT, entered).await.unwrap().unwrap();
            let stopping = match invalidation {
                Invalidation::Off => {
                    set(&node, false).await;
                    None
                }
                Invalidation::OffThenOn => {
                    set(&node, false).await;
                    set(&node, true).await;
                    None
                }
                Invalidation::Stop => {
                    let owner = owner.clone();
                    let id = original.id;
                    let stopping = tokio::spawn(async move { owner.kill(id).await });
                    tokio::time::timeout(WAIT, cancelled.cancelled())
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
                .read(original.id, EventSeq::ZERO, EVENT_PAGE)
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
async fn caller_loss_during_the_actual_settings_write_does_not_cancel_saved_off_apply() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&node, true).await;
            let settings = &node.secure.service.0.settings;
            let (entered, release) = settings.pause_write();
            let caller = {
                let settings = Arc::clone(settings);
                tokio::spawn(async move { settings.change(change(false).into()).await })
            };
            entered.await.unwrap();
            caller.abort();
            assert!(caller.await.unwrap_err().is_cancelled());
            let _barrier = owner.call(vessel_proto::GetNode).await.unwrap();
            release.send(()).unwrap();
            let effective = settings.current().await;
            assert!(!effective.restart_recovery.enabled);
            assert!(
                !f.data
                    .read_settings_record()
                    .unwrap()
                    .unwrap()
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
async fn caller_loss_after_saved_off_keeps_cleanup_ahead_of_the_next_on() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&node, true).await;
            let settings = &node.secure.service.0.settings;
            let (_, generation) = settings.policy().snapshot();
            let (entered, release) = settings.pause_cleanup();
            let caller = {
                let settings = Arc::clone(settings);
                tokio::spawn(async move { settings.change(change(false).into()).await })
            };
            entered.await.unwrap();
            assert!(
                !f.data
                    .read_settings_record()
                    .unwrap()
                    .unwrap()
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
                let settings = Arc::clone(settings);
                tokio::spawn(async move { settings.change(change(true).into()).await })
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

mod restart;

#[cfg(target_os = "linux")]
mod crash;

mod grants;

mod load;
use load::LoadGate;
mod order;

mod after_admission;
