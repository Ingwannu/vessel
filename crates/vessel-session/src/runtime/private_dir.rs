//! Every launch of a structured session carries the node's private
//! directory (`RuntimeConfig::private_dir`, `AgentSetup::private_dir`, ADR
//! 0057's launch step), and a probe carries none. Each site is checked on a
//! running session through the actor's own launch: a new session
//! (`launch`; a fork is the same setup), one started again (`again`), a
//! move (`agent::move_profile`), and the two that reuse the session's
//! launch, a wake and a rewind. A listing has no setup at all
//! (`vessel_agent::history_with` takes none).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;
use vessel_harness::fakes::{Role, claude_project, install};
use vessel_harness::{HarnessEnv, Lookup, Profile, parse_profiles};
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, HarnessKind, PromptQueue, ResolvedMcpServers,
    SessionEvent, SessionInput, SessionMode, SessionSpec, SessionStatus, TurnPoint,
};
use vessel_testkit::Fake;

use super::agent::probe_setup;
use super::{Created, RuntimeConfig, SessionRuntime};
use crate::again::{Ended, Relaunch, Reopening};
use crate::handle::SessionHandle;
use crate::moving::{Control, ProfileMove};
use crate::{
    DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, LaunchOptions,
};

/// Longest any one of these tests may take.
const SAFETY_NET: Duration = Duration::from_secs(60);

struct Fixture {
    dir: tempfile::TempDir,
    runtime: SessionRuntime,
}

impl Fixture {
    /// A runtime whose search path holds `fake-acp` as Claude's adapter,
    /// homes and a private directory of its own.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        drop(
            install(
                &bin,
                HarnessKind::ClaudeCode,
                Role::Structured,
                &Fake::Acp.path().unwrap(),
            )
            .unwrap(),
        );
        std::fs::create_dir(dir.path().join("private")).unwrap();
        std::fs::create_dir(dir.path().join("work")).unwrap();
        let runtime = SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: Lookup::new([bin]),
                harness_env: HarnessEnv::new(dir.path().to_owned()),
                ring_capacity: DEFAULT_RING_CAPACITY,
                durable_capacity: DEFAULT_DURABLE_CAPACITY,
                agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                park_after: None,
                private_dir: Some(dir.path().join("private")),
            },
            CancellationToken::new(),
        );
        Self { dir, runtime }
    }

    fn private(&self) -> PathBuf {
        self.dir.path().join("private")
    }

    fn work(&self) -> PathBuf {
        self.dir.path().join("work")
    }

    fn spec(&self) -> SessionSpec {
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
        .with_cwd(self.work())
    }

    /// A running session under `profile` (`None`: the default), its
    /// records drained as the node's persistence would.
    async fn running(&self, profile: Option<Profile>) -> SessionHandle {
        let spec = match &profile {
            Some(profile) => self.spec().with_profile(profile.name().clone()),
            None => self.spec(),
        };
        let Created {
            handle, durable, ..
        } = self
            .runtime
            .create_with(
                spec,
                LaunchOptions {
                    profile,
                    ..LaunchOptions::default()
                },
            )
            .await
            .unwrap();
        drain(durable);
        until_running(&handle).await;
        handle
    }
}

/// Take a session's durable records as the node does, so it never waits.
fn drain(mut durable: tokio::sync::mpsc::Receiver<Arc<EventRecord>>) {
    drop(tokio::spawn(async move {
        while durable.recv().await.is_some() {}
    }));
}

async fn until_running(handle: &SessionHandle) {
    let mut info = handle.watch_info();
    while info.borrow_and_update().status != SessionStatus::Running {
        info.changed().await.unwrap();
    }
}

/// The private directory `handle`'s current launch carries, as its actor
/// holds it.
async fn carried(handle: &SessionHandle) -> Option<PathBuf> {
    let (reply, answer) = oneshot::channel();
    let control = handle.0.control.clone().unwrap();
    control.send(Control::PrivateDir(reply)).await.unwrap();
    answer.await.unwrap()
}

async fn within<F: Future>(test: F) -> F::Output {
    vessel_testkit::within(SAFETY_NET, Box::pin(test))
        .await
        .expect("test timed out")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_new_session_and_one_started_again_carry_the_nodes_directory() {
    within(async {
        let fixture = Fixture::new();
        let handle = fixture.running(None).await;
        assert_eq!(carried(&handle).await, Some(fixture.private()));

        let id = handle.id();
        handle.kill();
        let info = handle.ended().await;
        drop(handle);
        let _removed = fixture.runtime.remove(id).unwrap();
        let ended = Ended {
            info,
            choices: Vec::new(),
            queue: PromptQueue::default(),
            usage: None,
            attention: None,
            moved_through: BTreeSet::new(),
        };
        let Reopening {
            created: Created {
                handle, durable, ..
            },
            gate,
        } = fixture
            .runtime
            .start_again(ended, Relaunch::default())
            .await
            .unwrap();
        drain(durable);
        gate.open();
        until_running(&handle).await;
        assert_eq!(carried(&handle).await, Some(fixture.private()));
        handle.kill();
        let _ended = handle.ended().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_move_carries_it_and_a_wake_keeps_it() {
    within(async {
        let fixture = Fixture::new();
        let homes = fixture.dir.path();
        let profiles = parse_profiles(
            &format!(
                "[profiles.claude-a]\nharness = \"claude\"\nenv = {{ CLAUDE_CONFIG_DIR = '{a}' }}\n\
                 [profiles.claude-b]\nharness = \"claude\"\nenv = {{ CLAUDE_CONFIG_DIR = '{b}' }}\n",
                a = homes.join(".claude-a").display(),
                b = homes.join(".claude-b").display(),
            ),
            Some(homes),
        )
        .unwrap();
        let named = |name: &str| profiles.get(&name.parse().unwrap()).cloned().unwrap();
        let (a, b) = (named("claude-a"), named("claude-b"));
        let env = HarnessEnv::new(homes.to_owned());
        let (home_a, home_b) = (env.home(&a).unwrap(), env.home(&b).unwrap());
        let handle = fixture.running(Some(a.clone())).await;
        // The transcript a move links into the other home.
        let conversation = handle.info().harness_session.unwrap();
        let project = claude_project(&home_a, &fixture.work()).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join(format!("{conversation}.jsonl")), "{}\n").unwrap();
        std::fs::create_dir_all(&home_b).unwrap();

        let moved = fixture
            .runtime
            .move_profile(
                handle.id(),
                ProfileMove {
                    limits: crate::LimitAssociation::Unavailable,
                    profile: b.clone(),
                    from_home: home_a,
                    to_home: home_b,
                    mcp_servers: ResolvedMcpServers::new(),
                    notices: Vec::new(),
                    force: false,
                    model_proxy: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(moved.profile.as_ref(), Some(b.name()));
        assert_eq!(carried(&handle).await, Some(fixture.private()));

        // A park and its wake restart the agent with the session's launch.
        let _parked = handle.park().await.unwrap();
        let _woken = handle.wake().await.unwrap();
        assert_eq!(carried(&handle).await, Some(fixture.private()));
        handle.kill();
        let _ended = handle.ended().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rewind_keeps_it() {
    within(async {
        let fixture = Fixture::new();
        let handle = fixture.running(None).await;
        let follow = handle.follow(EventSeq::ZERO).unwrap();
        let mut live = follow.live;
        handle
            .send(SessionInput::Prompt {
                content: vec![ContentBlock::Text {
                    text: "plain".to_owned(),
                }],
            })
            .await
            .unwrap();
        let (after, point) = turn_point(&mut live).await;
        let _rewound = handle.rewind(point, after, 0).await.unwrap();
        assert_eq!(carried(&handle).await, Some(fixture.private()));
        handle.kill();
        let _ended = handle.ended().await;
    })
    .await;
}

/// The first turn's end that names a point, and its record's place.
async fn turn_point(live: &mut broadcast::Receiver<Arc<EventRecord>>) -> (EventSeq, TurnPoint) {
    loop {
        let record = live.recv().await.unwrap();
        if let SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                point: Some(point), ..
            },
        } = &record.event
        {
            return (record.seq, point.clone());
        }
    }
}

#[test]
fn a_probe_carries_none() {
    let setup = probe_setup(Path::new("/work").to_path_buf(), Duration::from_secs(1));
    assert_eq!(setup.private_dir, None);
    assert!(setup.mcp_servers.is_empty());
}

mod stop_failure;

mod preparation;

mod registration;

mod version_admission;

mod prepared_view;
