//! What the session runtime's tests share: a runtime whose search path
//! holds `fake-acp` as Claude's adapter, and a session's log read as it
//! comes. Each test file takes it with `#[path]`, as the node's tests take
//! theirs.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;
use vessel_harness::Lookup;
use vessel_harness::fakes::{Role, install};
use vessel_proto::{
    AgentEvent, ChoiceValue, ContentBlock, EventRecord, EventSeq, HarnessKind, OptionChoice,
    OptionId, OptionValueId, SessionEvent, SessionInput, SessionMode, SessionOption, SessionSpec,
    SessionTitle, StopReason,
};
use vessel_session::{
    DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, LaunchOptions,
    RuntimeConfig, SessionHandle, SessionRuntime,
};
use vessel_testkit::Fake;

const SAFETY_NET: Duration = Duration::from_secs(30);

pub(crate) async fn within<F: Future>(test: F) -> F::Output {
    vessel_testkit::within(SAFETY_NET, test)
        .await
        .expect("test timed out")
}

/// A runtime whose search path holds `fake-acp` as Claude's adapter and
/// `fake-tui` as Claude.
pub(crate) struct Fixture {
    pub(crate) runtime: SessionRuntime,
    pub(crate) bin: PathBuf,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        Self::parking_after(None)
    }

    /// A runtime that parks idle agents after `after` (protocol 12).
    pub(crate) fn parking_after(after: Option<Duration>) -> Self {
        Self::configured(after, None)
    }

    pub(crate) fn checkpoint_hooks(hooks: vessel_session::CheckpointHooks) -> Self {
        Self::configured(None, Some(hooks))
    }

    pub(crate) fn startup_hooks(hooks: vessel_session::StartupHooks) -> Self {
        Self::configured_with(None, None, Some(hooks))
    }
    fn configured(after: Option<Duration>, hooks: Option<vessel_session::CheckpointHooks>) -> Self {
        Self::configured_with(after, hooks, None)
    }
    fn configured_with(
        after: Option<Duration>,
        hooks: Option<vessel_session::CheckpointHooks>,
        starts: Option<vessel_session::StartupHooks>,
    ) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let bin =
            std::env::temp_dir().join(format!("vessel-session-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&bin).unwrap();
        drop(
            install(
                &bin,
                HarnessKind::ClaudeCode,
                Role::Structured,
                &Fake::Acp.path().unwrap(),
            )
            .unwrap(),
        );
        drop(
            install(
                &bin,
                HarnessKind::ClaudeCode,
                Role::Terminal,
                &Fake::Tui.path().unwrap(),
            )
            .unwrap(),
        );
        let runtime = SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: Lookup::new([bin.clone()]),
                // Never the machine's own account.
                harness_env: vessel_harness::HarnessEnv::new(std::env::temp_dir()),
                ring_capacity: DEFAULT_RING_CAPACITY,
                durable_capacity: DEFAULT_DURABLE_CAPACITY,
                agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                park_after: after,
                private_dir: None,
            },
            CancellationToken::new(),
        );
        let runtime = match hooks {
            Some(hooks) => runtime.with_checkpoint_hooks(hooks),
            None => runtime,
        };
        let runtime = match starts {
            Some(hooks) => runtime.with_startup_hooks(hooks),
            None => runtime,
        };
        Self { runtime, bin }
    }

    /// Start `spec`; its durable records are drained so it never stalls.
    pub(crate) async fn start(&self, spec: SessionSpec) -> SessionHandle {
        self.start_with(spec, LaunchOptions::default()).await
    }

    /// [`start`](Self::start) with `options`.
    pub(crate) async fn start_with(
        &self,
        spec: SessionSpec,
        options: LaunchOptions,
    ) -> SessionHandle {
        let created = self.runtime.create_with(spec, options).await.unwrap();
        let mut durable = created.durable;
        drop(tokio::spawn(async move {
            while durable.recv().await.is_some() {}
        }));
        created.handle
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.bin));
    }
}

pub(crate) fn structured(options: Vec<OptionChoice>) -> SessionSpec {
    SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options,
        },
    )
    .with_cwd(std::env::temp_dir())
}

pub(crate) fn select(id: &str, value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}

pub(crate) fn title(text: &str) -> SessionTitle {
    text.parse().unwrap()
}

pub(crate) fn prompt(text: &str) -> SessionInput {
    SessionInput::Prompt {
        content: vec![ContentBlock::Text {
            text: text.to_owned(),
        }],
    }
}

/// A session's records, read from the start as they come.
pub(crate) struct Log {
    pub(crate) live: tokio::sync::broadcast::Receiver<Arc<EventRecord>>,
    pub(crate) records: Vec<Arc<EventRecord>>,
}

impl Log {
    pub(crate) fn of(handle: &SessionHandle) -> Self {
        Self::from(handle, EventSeq::ZERO)
    }

    /// From record `from` on (a session started again holds only its new
    /// run's records).
    pub(crate) fn from(handle: &SessionHandle, from: EventSeq) -> Self {
        let follow = handle.follow(from).unwrap();
        Self {
            records: follow.cached.clone(),
            live: follow.live,
        }
    }

    /// Read until a record matches (one already read counts).
    pub(crate) async fn until(&mut self, wanted: impl Fn(&SessionEvent) -> bool) {
        if self.records.iter().any(|record| wanted(&record.event)) {
            return;
        }
        self.next(wanted).await;
    }

    /// Read until a record at or after `from` (an index into `records`)
    /// matches.
    pub(crate) async fn since(&mut self, from: usize, wanted: impl Fn(&SessionEvent) -> bool) {
        let read = self.records.get(from..).unwrap_or_default();
        if read.iter().any(|record| wanted(&record.event)) {
            return;
        }
        self.next(wanted).await;
    }

    /// Read until a record not read yet matches.
    pub(crate) async fn next(&mut self, wanted: impl Fn(&SessionEvent) -> bool) {
        loop {
            let record = match self.live.recv().await {
                Ok(record) => record,
                Err(RecvError::Closed) => panic!("ended first: {:?}", self.records),
                Err(RecvError::Lagged(n)) => panic!("lagged by {n}"),
            };
            let found = wanted(&record.event);
            self.records.push(record);
            if found {
                return;
            }
        }
    }

    pub(crate) async fn turn_over(&mut self) {
        self.until(|event| {
            matches!(
                event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded {
                        reason: StopReason::EndTurn | StopReason::Cancelled,
                        usage: None,
                        ..
                    }
                }
            )
        })
        .await;
    }

    pub(crate) fn titles(&self) -> Vec<&str> {
        self.records
            .iter()
            .filter_map(|record| {
                if let SessionEvent::Renamed { title, .. } = &record.event {
                    Some(title.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn reports(&self) -> Vec<&[SessionOption]> {
        self.records
            .iter()
            .filter_map(|record| {
                if let SessionEvent::OptionsChanged { options } = &record.event {
                    Some(options.as_slice())
                } else {
                    None
                }
            })
            .collect()
    }
}

pub(crate) fn mode(options: &[SessionOption]) -> Option<&str> {
    options
        .iter()
        .find(|option| option.id.as_str() == "mode")
        .and_then(SessionOption::current_name)
}
