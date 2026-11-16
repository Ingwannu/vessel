//! Hold actual resolved lookups; closing admission must precede any process call.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;

use tokio::sync::oneshot;
use vessel_harness::fakes::{Role, install};
use vessel_harness::{HarnessEnv, Lookup, Profile, parse_profiles};
use vessel_proto::{HarnessKind, SessionMode, SessionSpec, TerminalSize};
use vessel_testkit::Fake;

use super::*;
use crate::runtime::{LaunchOptions, SessionRuntime};
use crate::{
    DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, RuntimeConfig,
};

const ADMISSION_CASE_BUDGET: Duration = Duration::from_secs(30);

async fn phase<F: std::future::Future>(name: &str, future: F) -> F::Output {
    tokio::time::timeout(ADMISSION_CASE_BUDGET, future)
        .await
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[derive(Debug, Default)]
pub(in crate::runtime) struct LookupGateSlot(Mutex<Option<LookupGate>>);

impl LookupGateSlot {
    pub(in crate::runtime) fn take(&self) -> Option<LookupGate> {
        self.0.lock().unwrap().take()
    }

    fn install(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, observing) = oneshot::channel();
        let (release, released) = oneshot::channel();
        *self.0.lock().unwrap() = Some(LookupGate { entered, released });
        (observing, release)
    }
}

#[derive(Debug)]
pub(in crate::runtime) struct LookupGate {
    entered: oneshot::Sender<()>,
    released: oneshot::Receiver<()>,
}

impl LookupGate {
    pub(in crate::runtime) fn pause(self) -> Result<(), SessionError> {
        // Abandoning a failed fixture releases the actual blocking reader while
        // refusing a late child; observer loss never becomes successful lookup.
        self.entered
            .send(())
            .map_err(|()| SessionError::ShuttingDown)?;
        self.released
            .blocking_recv()
            .map_err(|_closed| SessionError::ShuttingDown)
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Structured,
    Terminal,
}

struct Fixture {
    _root: tempfile::TempDir,
    runtime: Arc<SessionRuntime>,
    node_stop: CancellationToken,
    actor_stop: CancellationToken,
    profile: Profile,
    marker: PathBuf,
    cwd: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Failure still closes the original owners. Their existing child Drop
        // and task ownership remain in force; no PID signal or detached reaper.
        self.node_stop.cancel();
        self.actor_stop.cancel();
    }
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        let home = root.path().join("home");
        std::fs::create_dir(&bin).unwrap();
        std::fs::create_dir(&home).unwrap();
        for role in [Role::Structured, Role::Terminal] {
            let _installed = install(
                &bin,
                HarnessKind::ClaudeCode,
                role,
                &Fake::Tool.path().unwrap(),
            )
            .unwrap();
        }
        let marker = root.path().join("actual-launch.log");
        let path = serde_json::to_string(&marker.to_string_lossy()).unwrap();
        let text = format!(
            "[profiles.observer]\nharness = \"claude\"\n[profiles.observer.env]\nFAKE_TOOL_LOG = {path}\n"
        );
        let profiles = parse_profiles(&text, Some(&home)).unwrap();
        let profile = profiles.get(&"observer".parse().unwrap()).unwrap().clone();
        let node_stop = CancellationToken::new();
        let actor_stop = CancellationToken::new();
        let runtime = SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: Lookup::new([bin]),
                harness_env: HarnessEnv::new(home),
                ring_capacity: DEFAULT_RING_CAPACITY,
                durable_capacity: DEFAULT_DURABLE_CAPACITY,
                agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                park_after: None,
                private_dir: None,
            },
            actor_stop.clone(),
        )
        .with_admission(node_stop.clone());
        let cwd = root.path().to_owned();
        Self {
            _root: root,
            runtime: Arc::new(runtime),
            node_stop,
            actor_stop,
            profile,
            marker,
            cwd,
        }
    }

    fn spec(&self, mode: Mode) -> SessionSpec {
        let mode = match mode {
            Mode::Structured => SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
            Mode::Terminal => SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        };
        SessionSpec::new(HarnessKind::ClaudeCode, mode)
            .with_cwd(self.cwd.clone())
            .with_profile(self.profile.name().clone())
    }

    fn options(&self) -> LaunchOptions {
        LaunchOptions {
            profile: Some(self.profile.clone()),
            ..LaunchOptions::default()
        }
    }
}

async fn held_lookup(mode: Mode) {
    vessel_testkit::within(ADMISSION_CASE_BUDGET, async {
        let f = Fixture::new();
        let (entered, release) = f.runtime.lookup_gate.install();
        let runtime = Arc::clone(&f.runtime);
        let spec = f.spec(mode);
        let options = f.options();
        let mut create = tokio::spawn(async move { runtime.create_with(spec, options).await });
        phase("actual lookup reached", entered).await.unwrap();
        // Actor shutdown is intentionally deferred, as it is during Settings drain.
        f.node_stop.cancel();
        assert!(!f.actor_stop.is_cancelled());
        release.send(()).unwrap();
        let result = phase("held create joins", &mut create).await.unwrap();
        if let Ok(mut created) = result {
            // Settle a regression's owned child before reporting it, so a failing
            // test cannot leave a process behind its assertion.
            phase("owned fake durable stream ends", async {
                while created.durable.recv().await.is_some() {}
            })
            .await;
            phase("owned runtime shutdown", f.runtime.shutdown()).await;
            panic!("a lookup admitted before stop started a process after stop");
        }
        assert!(matches!(result, Err(SessionError::ShuttingDown)));
        phase("owned runtime shutdown", f.runtime.shutdown()).await;
        assert!(f.runtime.list().is_empty());
        assert!(
            !f.marker.exists(),
            "the real fake program was never invoked"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn structured_lookup_held_across_node_stop_never_spawns() {
    held_lookup(Mode::Structured).await;
}

#[tokio::test]
async fn terminal_lookup_held_across_node_stop_never_spawns() {
    held_lookup(Mode::Terminal).await;
}

#[tokio::test]
async fn the_same_fake_and_profile_record_actual_launches_while_admission_is_open() {
    vessel_testkit::within(ADMISSION_CASE_BUDGET, async {
        for mode in [Mode::Structured, Mode::Terminal] {
            let f = Fixture::new();
            let mut created = phase(
                "open admission creates actual fake",
                f.runtime.create_with(f.spec(mode), f.options()),
            )
            .await
            .unwrap();
            phase("owned fake durable stream ends", async {
                while created.durable.recv().await.is_some() {}
            })
            .await;
            assert!(
                f.marker.exists(),
                "positive control observes an actual child invocation"
            );
            assert!(!std::fs::read(&f.marker).unwrap().is_empty());
            phase("owned runtime shutdown", f.runtime.shutdown()).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn observers_wake_on_either_original_token_including_already_closed_sources() {
    vessel_testkit::within(ADMISSION_CASE_BUDGET, async {
        let runtime = CancellationToken::new();
        let node = CancellationToken::new();
        let admission = LaunchAdmission::new(runtime.clone(), Some(node.clone()));
        assert!(admission.check().is_ok());
        let mut wait = Box::pin(admission.cancelled());
        std::future::poll_fn(|context| {
            assert!(matches!(wait.as_mut().poll(context), Poll::Pending));
            Poll::Ready(())
        })
        .await;
        node.cancel();
        phase("node-stop observer wakes", wait).await;
        assert!(!runtime.is_cancelled());
        assert!(matches!(admission.check(), Err(SessionError::ShuttingDown)));
        phase("already closed node-stop observer", admission.cancelled()).await;
        let standalone_stop = CancellationToken::new();
        let standalone = LaunchAdmission::new(standalone_stop.clone(), None);
        assert!(standalone.check().is_ok());
        standalone_stop.cancel();
        phase("standalone runtime observer", standalone.cancelled()).await;
        assert!(matches!(
            standalone.check(),
            Err(SessionError::ShuttingDown)
        ));
    })
    .await
    .unwrap();
}
