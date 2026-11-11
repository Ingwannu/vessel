//! Protocol 9 in the session runtime, with `fake-acp` as Claude's adapter:
//! the options a session logs and changes, and its title.
//!
//! Tests wait on records, never on time; the timeout only turns a hang
//! into a failure.

use vessel_proto::{
    ErrorCode, HarnessKind, SessionEvent, SessionMode, SessionSpec, SessionStatus, TerminalSize,
};
use vessel_session::SessionError;

use support::{Fixture, Log, mode, prompt, select, structured, title, within};

#[cfg(test)]
mod support {
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
        OptionId, OptionValueId, SessionEvent, SessionInput, SessionMode, SessionOption,
        SessionSpec, SessionTitle, StopReason,
    };
    use vessel_session::{
        DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, RuntimeConfig,
        SessionHandle, SessionRuntime,
    };
    use vessel_testkit::Fake;

    const SAFETY_NET: Duration = Duration::from_secs(30);

    pub(super) async fn within<F: Future>(test: F) -> F::Output {
        vessel_testkit::within(SAFETY_NET, test)
            .await
            .expect("test timed out")
    }

    /// A runtime whose search path holds `fake-acp` as Claude's adapter and
    /// `fake-tui` as Claude.
    pub(super) struct Fixture {
        pub(super) runtime: SessionRuntime,
        pub(super) bin: PathBuf,
    }

    impl Fixture {
        pub(super) fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let bin = std::env::temp_dir()
                .join(format!("vessel-options-test-{}-{n}", std::process::id()));
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
                },
                CancellationToken::new(),
            );
            Self { runtime, bin }
        }

        /// Start `spec`; its durable records are drained so it never stalls.
        pub(super) async fn start(&self, spec: SessionSpec) -> SessionHandle {
            let created = self.runtime.create(spec).await.unwrap();
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

    pub(super) fn structured(options: Vec<OptionChoice>) -> SessionSpec {
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

    pub(super) fn select(id: &str, value: &str) -> OptionChoice {
        OptionChoice {
            id: OptionId::new(id),
            value: ChoiceValue::Select {
                value: OptionValueId::new(value),
            },
        }
    }

    pub(super) fn title(text: &str) -> SessionTitle {
        text.parse().unwrap()
    }

    pub(super) fn prompt(text: &str) -> SessionInput {
        SessionInput::Prompt {
            content: vec![ContentBlock::Text {
                text: text.to_owned(),
            }],
        }
    }

    /// A session's records, read from the start as they come.
    pub(super) struct Log {
        pub(super) live: tokio::sync::broadcast::Receiver<Arc<EventRecord>>,
        pub(super) records: Vec<Arc<EventRecord>>,
    }

    impl Log {
        pub(super) fn of(handle: &SessionHandle) -> Self {
            let follow = handle.follow(EventSeq::ZERO).unwrap();
            Self {
                records: follow.cached.clone(),
                live: follow.live,
            }
        }

        /// Read until a record matches (one already read counts).
        pub(super) async fn until(&mut self, wanted: impl Fn(&SessionEvent) -> bool) {
            if self.records.iter().any(|record| wanted(&record.event)) {
                return;
            }
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

        pub(super) async fn turn_over(&mut self) {
            self.until(|event| {
                matches!(
                    event,
                    SessionEvent::Agent {
                        event: AgentEvent::TurnEnded {
                            reason: StopReason::EndTurn | StopReason::Cancelled
                        }
                    }
                )
            })
            .await;
        }

        pub(super) fn titles(&self) -> Vec<&str> {
            self.records
                .iter()
                .filter_map(|record| {
                    if let SessionEvent::Renamed { title } = &record.event {
                        Some(title.as_str())
                    } else {
                        None
                    }
                })
                .collect()
        }

        pub(super) fn reports(&self) -> Vec<&[SessionOption]> {
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

    pub(super) fn mode(options: &[SessionOption]) -> Option<&str> {
        options
            .iter()
            .find(|option| option.id.as_str() == "mode")
            .and_then(SessionOption::current_name)
    }
}

/// An untitled session takes the first line of its first prompt; a rename
/// replaces it; later prompts do not.
#[tokio::test]
async fn the_first_prompt_names_a_session_and_a_rename_replaces_it() {
    within(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        assert_eq!(handle.info().title, None);
        let mut log = Log::of(&handle);
        // The fake ends a turn at once only for `plain`; the first line
        // and the cut are the protocol's own tests (`SessionTitle`).
        handle.send(prompt("plain")).await.unwrap();
        log.until(|event| matches!(event, SessionEvent::Renamed { .. }))
            .await;
        assert_eq!(handle.info().title, Some(title("plain")));
        log.turn_over().await;

        let renamed = handle.rename(title("Release notes")).await.unwrap();
        assert_eq!(renamed.title, Some(title("Release notes")));
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        log.until(|event| matches!(event, SessionEvent::Renamed { title } if title.as_str() == "Release notes"))
            .await;
        assert_eq!(log.titles(), ["plain", "Release notes"]);
        assert_eq!(handle.info().title, Some(title("Release notes")));

        handle.kill();
        let _end = handle.ended().await;
        let error = handle.rename(title("Too late")).await.unwrap_err();
        assert!(matches!(error, SessionError::Ended), "{error:?}");
    })
    .await;
}

/// A session created with a title keeps it: its prompts do not rename it.
#[tokio::test]
async fn a_given_title_is_kept() {
    within(async {
        let fixture = Fixture::new();
        let handle = fixture
            .start(structured(Vec::new()).with_title(title("Mine")))
            .await;
        assert_eq!(handle.info().title, Some(title("Mine")));
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        assert!(log.titles().is_empty(), "{:?}", log.titles());
        handle.kill();
        let _end = handle.ended().await;
    })
    .await;
}

/// The options are in the log after the handshake, as chosen; a change is
/// in the log too, and is what the session carries on.
#[tokio::test]
async fn a_session_logs_its_options_and_their_changes() {
    within(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(vec![select("mode", "plan")])).await;
        // Before any report, what the spec chose.
        let mut log = Log::of(&handle);
        log.until(|event| matches!(event, SessionEvent::OptionsChanged { .. }))
            .await;
        assert_eq!(log.reports().first().and_then(|options| mode(options)), Some("Plan"));
        assert_eq!(mode(&handle.options().await.unwrap()), Some("Plan"));

        let changed = handle.set_option(select("mode", "default")).await.unwrap();
        assert_eq!(mode(&changed), Some("Default"));
        log.until(|event| matches!(event, SessionEvent::OptionsChanged { options } if mode(options) == Some("Default")))
            .await;
        assert!(handle.choices().contains(&select("mode", "default")));

        let error = handle
            .set_option(select("mode", "locked"))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::Conflict, "the agent's refusal");
        let error = handle.set_option(select("mode", "yolo")).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidRequest, "not offered");
        handle.kill();
        let _end = handle.ended().await;
    })
    .await;
}

/// A choice the agent does not offer fails the session at its start.
#[tokio::test]
async fn a_choice_not_offered_fails_the_session() {
    within(async {
        let fixture = Fixture::new();
        let handle = fixture
            .start(structured(vec![select("mode", "yolo")]))
            .await;
        let end = handle.ended().await;
        let SessionStatus::Failed { message } = &end.status else {
            panic!("failed: {:?}", end.status);
        };
        assert!(
            message.contains("option `mode` has no value `yolo`"),
            "{message}"
        );
    })
    .await;
}

/// A terminal session has no options to read or set, and can be renamed.
#[tokio::test]
async fn a_terminal_session_has_no_options_but_a_title() {
    within(async {
        let fixture = Fixture::new();
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        )
        .with_cwd(std::env::temp_dir());
        let handle = fixture.start(spec).await;
        let error = handle.options().await.unwrap_err();
        assert!(matches!(error, SessionError::NoOptions), "{error:?}");
        assert_eq!(error.code(), ErrorCode::Unsupported);
        assert!(handle.choices().is_empty());
        let renamed = handle.rename(title("A shell")).await.unwrap();
        assert_eq!(renamed.title, Some(title("A shell")));
        handle.kill();
        let _end = handle.ended().await;
    })
    .await;
}
