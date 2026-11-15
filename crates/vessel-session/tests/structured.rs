//! The session runtime end to end, in structured mode, with `fake-acp`
//! installed as `claude-agent-acp`.
//!
//! Tests wait on events, never on time; the timeout only turns a hang into
//! a failure.

use vessel_proto::{
    Activity, AgentEvent, ContentBlock, DisplayStatus, ErrorCode, HarnessKind, HarnessSessionId,
    KanbanColumn, KeptMarks, ModelId, PermissionOptionId, PermissionOutcome, SessionBoard,
    SessionEvent, SessionInput, SessionMode, SessionOption, SessionSpec, SessionStatus, StopReason,
    TerminalInput, TerminalSize,
};

use support::{Fixture, Follower, board_until, collect, within};

#[cfg(test)]
mod support {
    use std::collections::VecDeque;
    use std::future::Future;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    use tokio::sync::broadcast::error::RecvError;
    use tokio::sync::mpsc;
    use tokio::task::JoinHandle;
    use tokio_util::sync::CancellationToken;
    use vessel_harness::Lookup;
    use vessel_harness::fakes::{Role, install};
    use vessel_proto::{
        DisplayStatus, EventRecord, EventSeq, HarnessKind, SessionBoard, SessionEvent, SessionInfo,
        SessionStatus,
    };
    use vessel_session::{
        DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, Follow,
        RuntimeConfig, SessionHandle, SessionRuntime,
    };
    use vessel_testkit::Fake;

    const SAFETY_NET: Duration = Duration::from_secs(30);

    /// The safety net starts after the fakes are built: the first build of
    /// a run can take a minute (see `vessel_testkit::within`).
    pub(super) async fn within<F: Future>(test: F) -> F::Output {
        vessel_testkit::within(SAFETY_NET, test)
            .await
            .expect("test timed out")
    }

    /// A runtime whose search path holds `fake-acp` as Claude's adapter.
    pub(super) struct Fixture {
        pub(super) runtime: SessionRuntime,
        bin: PathBuf,
    }

    impl Fixture {
        pub(super) fn new() -> Self {
            Self::with_adapter(|bin, acp| {
                drop(install(bin, HarnessKind::ClaudeCode, Role::Structured, acp).unwrap());
            })
        }

        /// Claude's adapter never answers `initialize` (fake-acp with
        /// `FAKE_ACP_HANG`, behind a wrapper that sets it).
        pub(super) fn with_stuck_adapter() -> Self {
            Self::with_adapter(|bin, acp| {
                #[cfg(unix)]
                {
                    let wrapper = bin.join("claude-agent-acp");
                    vessel_testkit::write_executable(
                        &wrapper,
                        &format!(
                            "#!/bin/sh\nFAKE_ACP_HANG=initialize exec '{}' \"$@\"\n",
                            acp.display()
                        ),
                    )
                    .unwrap();
                }
                #[cfg(windows)]
                std::fs::write(
                    bin.join("claude-agent-acp.cmd"),
                    format!(
                        "@echo off\r\nset FAKE_ACP_HANG=initialize\r\n\"{}\" %*\r\nexit /b %ERRORLEVEL%\r\n",
                        acp.display()
                    ),
                )
                .unwrap();
            })
        }

        fn with_adapter(adapter: impl FnOnce(&std::path::Path, &std::path::Path)) -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let bin = std::env::temp_dir()
                .join(format!("vessel-structured-test-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&bin).unwrap();
            let acp = Fake::Acp.path().unwrap();
            adapter(&bin, &acp);
            let runtime = SessionRuntime::new(
                RuntimeConfig {
                    harness_lookup: Lookup::new([bin.clone()]),
                    // Never the machine's own account.
                    harness_env: vessel_harness::HarnessEnv::new(std::env::temp_dir()),
                    ring_capacity: DEFAULT_RING_CAPACITY,
                    durable_capacity: DEFAULT_DURABLE_CAPACITY,
                    agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                    park_after: None,
                    private_dir: None,
                },
                CancellationToken::new(),
            );
            Self { runtime, bin }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            drop(std::fs::remove_dir_all(&self.bin));
        }
    }

    pub(super) fn collect(
        mut durable: mpsc::Receiver<vessel_session::JournalEntry>,
    ) -> JoinHandle<Vec<Arc<EventRecord>>> {
        tokio::spawn(async move {
            let mut records = Vec::new();
            while let Some(record) = durable.recv().await {
                records.push(record.into_event().expect("records-only runtime"));
            }
            records
        })
    }

    /// Wait on the info watch until the board shows `phrase`, and return that
    /// board. Waits on the session's own notifications, never on a clock.
    pub(super) async fn board_until(
        watch: &mut tokio::sync::watch::Receiver<SessionInfo>,
        phrase: DisplayStatus,
    ) -> SessionBoard {
        let info = watch
            .wait_for(|info| info.board.as_ref().map(|board| board.display_status) == Some(phrase))
            .await
            .unwrap_or_else(|_closed| panic!("the session ended before {phrase:?}"));
        info.board.clone().unwrap()
    }

    /// Reads one session's records from the start.
    pub(super) struct Follower {
        cached: VecDeque<Arc<EventRecord>>,
        follow: Follow,
        pub(super) records: Vec<Arc<EventRecord>>,
    }

    impl Follower {
        pub(super) fn new(handle: &SessionHandle) -> Self {
            let follow = handle.follow(EventSeq::ZERO).unwrap();
            Self {
                cached: follow.cached.iter().cloned().collect(),
                follow,
                records: Vec::new(),
            }
        }

        pub(super) async fn next(&mut self) -> Option<SessionEvent> {
            let record = match self.cached.pop_front() {
                Some(record) => record,
                None => match self.follow.live.recv().await {
                    Ok(record) => record,
                    Err(RecvError::Closed) => return None,
                    Err(RecvError::Lagged(n)) => panic!("follower lagged by {n}"),
                },
            };
            self.records.push(Arc::clone(&record));
            Some(record.event.clone())
        }

        pub(super) async fn until(
            &mut self,
            wanted: impl Fn(&SessionEvent) -> bool,
        ) -> SessionEvent {
            loop {
                let Some(event) = self.next().await else {
                    panic!("session ended first: {:?}", self.records);
                };
                if wanted(&event) {
                    return event;
                }
            }
        }

        /// Read to the end; returns the final status.
        pub(super) async fn until_end(&mut self) -> SessionStatus {
            let mut last = None;
            while let Some(event) = self.next().await {
                if let SessionEvent::Status { status } = event {
                    last = Some(status);
                }
            }
            last.expect("every session ends with a status")
        }
    }
}

fn structured(model: Option<&str>) -> SessionSpec {
    SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: model.map(ModelId::new),
            resume: None,
            options: Vec::new(),
        },
    )
    .with_cwd(std::env::temp_dir())
}

fn prompt(text: &str) -> SessionInput {
    SessionInput::Prompt {
        content: vec![ContentBlock::Text {
            text: text.to_owned(),
        }],
    }
}

fn agent(wanted: fn(&AgentEvent) -> bool) -> impl Fn(&SessionEvent) -> bool {
    move |event| matches!(event, SessionEvent::Agent { event } if wanted(event))
}

const fn status(event: &SessionEvent) -> Option<&SessionStatus> {
    match event {
        SessionEvent::Status { status } => Some(status),
        SessionEvent::Terminal(_)
        | SessionEvent::Agent { .. }
        | SessionEvent::ProfileChanged { .. }
        | SessionEvent::ModelCall(_)
        | SessionEvent::OptionsChanged { .. }
        | SessionEvent::Renamed { .. }
        | SessionEvent::QueueChanged { .. }
        | SessionEvent::CommandsChanged { .. }
        | SessionEvent::UsageChanged { .. }
        | SessionEvent::Parked { .. }
        | SessionEvent::Woken
        | SessionEvent::Forked { .. }
        | SessionEvent::Rewound { .. } => None,
    }
}

#[tokio::test]
async fn status_follows_the_acp_handshake() {
    within(async {
        let fixture = Fixture::new();
        let created = fixture
            .runtime
            .create(structured(Some("fake-large")))
            .await
            .unwrap();
        let stored = collect(created.durable);
        let handle = created.handle;
        let mut follower = Follower::new(&handle);

        let running = follower
            .until(|event| status(event) == Some(&SessionStatus::Running))
            .await;
        assert_eq!(
            running,
            SessionEvent::Status {
                status: SessionStatus::Running
            }
        );
        let first: Vec<_> = follower
            .records
            .iter()
            .filter(|record| {
                // The commands an agent announces come when it sends them,
                // before or after its answer to `session/new` (Claude's
                // adapter sends them after); they are not the handshake.
                !matches!(
                    &record.event,
                    SessionEvent::Agent {
                        event: AgentEvent::HarnessLog { .. }
                    } | SessionEvent::CommandsChanged { .. }
                )
            })
            .map(|record| record.event.clone())
            .collect();
        // Protocol 9: the options as they stand after the handshake (the
        // model it asked for already selected) come right before `Ready`,
        // so a session seen running has its options in the log.
        let [starting, options, ready, running] = first.as_slice() else {
            panic!("four records before running: {first:?}");
        };
        // The fake's first conversation, its id its own (`fake-session-1-<tag>`).
        let SessionEvent::Agent {
            event:
                AgentEvent::Ready {
                    harness_session: conversation,
                    ..
                },
        } = ready
        else {
            panic!("ready: {ready:?}");
        };
        assert!(
            conversation.as_str().starts_with("fake-session-1-"),
            "{conversation}"
        );
        assert_eq!(
            (starting, ready, running),
            (
                &SessionEvent::Status {
                    status: SessionStatus::Starting
                },
                &SessionEvent::Agent {
                    event: AgentEvent::Ready {
                        harness_session: conversation.clone(),
                        abilities: Some(vessel_proto::AgentAbilities {
                            steer: true,
                            resume: true,
                            image: true,
                            audio: false,
                            fork: true,
                        })
                    }
                },
                &SessionEvent::Status {
                    status: SessionStatus::Running
                },
            )
        );
        let SessionEvent::OptionsChanged { options } = options else {
            panic!("the options before ready: {options:?}");
        };
        assert_eq!(
            SessionOption::summary(options),
            "Mode Default, Model Fake-large, Effort High, Fast off"
        );
        assert_eq!(handle.info().harness_session.as_ref(), Some(conversation));

        handle.kill();
        let end = follower.until_end().await;
        assert!(matches!(end, SessionStatus::Exited { .. }), "{end:?}");
        assert_eq!(stored.await.unwrap(), follower.records);
    })
    .await;
}

#[tokio::test]
async fn a_turn_with_a_permission_round_trip() {
    within(async {
        let fixture = Fixture::new();
        let created = fixture
            .runtime
            .create(structured(Some("fake-large")))
            .await
            .unwrap();
        let stored = collect(created.durable);
        let handle = created.handle;
        let mut follower = Follower::new(&handle);

        handle.send(prompt("hello")).await.unwrap();
        let SessionEvent::Agent {
            event: AgentEvent::PermissionRequested(request),
        } = follower
            .until(agent(|event| {
                matches!(event, AgentEvent::PermissionRequested(_))
            }))
            .await
        else {
            panic!("matched by the predicate above");
        };
        let allow = SessionInput::AnswerPermission {
            id: request.id,
            outcome: PermissionOutcome::Selected {
                option: PermissionOptionId::new("allow-once"),
            },
        };
        // Waiting on a permission is the middle of a turn (what
        // `session.export` refuses without `force`).
        assert!(handle.turn_running().await.unwrap());
        handle.send(allow.clone()).await.unwrap();
        assert_eq!(
            handle.send(allow).await.unwrap_err().code(),
            ErrorCode::Conflict
        );
        assert_eq!(
            follower
                .until(agent(|event| matches!(event, AgentEvent::TurnEnded { .. })))
                .await,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded {
                    reason: StopReason::EndTurn,
                    usage: None,
                    point: None,
                }
            }
        );
        assert!(follower.records.iter().any(|record| record.event
            == SessionEvent::Agent {
                event: AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: "echo: hello (model fake-large)".to_owned()
                    }
                }
            }));

        assert!(!handle.turn_running().await.unwrap(), "between turns");

        let keys = SessionInput::Terminal {
            input: TerminalInput::Resize {
                size: TerminalSize::default(),
            },
        };
        assert_eq!(
            handle.send(keys).await.unwrap_err().code(),
            ErrorCode::Unsupported
        );

        handle.kill();
        let end = follower.until_end().await;
        assert!(matches!(end, SessionStatus::Exited { .. }), "{end:?}");
        assert_eq!(stored.await.unwrap(), follower.records);
    })
    .await;
}

#[tokio::test]
async fn prompts_sent_while_starting_wait_for_the_handshake() {
    within(async {
        let fixture = Fixture::new();
        let created = fixture.runtime.create(structured(None)).await.unwrap();
        let stored = collect(created.durable);
        let handle = created.handle;
        // Sent before the agent is ready: it waits, then runs.
        handle.send(prompt("plain")).await.unwrap();
        let mut follower = Follower::new(&handle);
        drop(
            follower
                .until(agent(|event| matches!(event, AgentEvent::TurnEnded { .. })))
                .await,
        );
        handle.kill();
        drop(follower.until_end().await);
        drop(stored.await.unwrap());
    })
    .await;
}

#[tokio::test]
async fn a_model_the_agent_does_not_offer_fails_the_session() {
    within(async {
        let fixture = Fixture::new();
        let created = fixture
            .runtime
            .create(structured(Some("gpt-imaginary")))
            .await
            .unwrap();
        let stored = collect(created.durable);
        let mut follower = Follower::new(&created.handle);
        let SessionStatus::Failed { message, .. } = follower.until_end().await else {
            panic!("a failed handshake fails the session");
        };
        assert!(
            message.contains("does not offer model `gpt-imaginary`"),
            "{message}"
        );
        assert!(
            !follower
                .records
                .iter()
                .any(|record| status(&record.event) == Some(&SessionStatus::Running)),
            "never running"
        );
        assert_eq!(stored.await.unwrap(), follower.records);
        assert!(matches!(
            created.handle.info().status,
            SessionStatus::Failed { .. }
        ));
    })
    .await;
}

#[tokio::test]
async fn resuming_replays_history_before_the_session_runs() {
    within(async {
        let fixture = Fixture::new();
        let mut spec = structured(None);
        spec.mode = SessionMode::Structured {
            model: None,
            resume: Some(HarnessSessionId::new("fake-session-7")),
            options: Vec::new(),
        };
        let created = fixture.runtime.create(spec).await.unwrap();
        let stored = collect(created.durable);
        let mut follower = Follower::new(&created.handle);
        drop(
            follower
                .until(|event| status(event) == Some(&SessionStatus::Running))
                .await,
        );
        let replay = SessionEvent::Agent {
            event: AgentEvent::MessageChunk {
                content: ContentBlock::Text {
                    text: "replayed fake-session-7".to_owned(),
                },
            },
        };
        assert!(follower.records.iter().any(|record| record.event == replay));
        assert_eq!(
            created.handle.info().harness_session,
            Some(HarnessSessionId::new("fake-session-7"))
        );
        fixture.runtime.shutdown().await;
        assert_eq!(follower.until_end().await, SessionStatus::Interrupted);
        drop(stored.await.unwrap());
    })
    .await;
}

/// Killed while still starting (here: an adapter stuck before `Ready`), a
/// structured session ends `Exited` like a killed terminal session, not
/// `Failed` as if it could not start.
#[tokio::test]
async fn a_session_killed_while_starting_ends_exited() {
    within(async {
        let fixture = Fixture::with_stuck_adapter();
        let created = fixture.runtime.create(structured(None)).await.unwrap();
        let stored = collect(created.durable);
        let mut follower = Follower::new(&created.handle);
        assert_eq!(
            follower.until(|event| status(event).is_some()).await,
            SessionEvent::Status {
                status: SessionStatus::Starting
            }
        );
        created.handle.kill();
        let end = follower.until_end().await;
        assert!(matches!(end, SessionStatus::Exited { .. }), "{end:?}");
        assert_eq!(stored.await.unwrap(), follower.records);
    })
    .await;
}

/// The board follows the agent (protocol 8, ADR 0019): working while it
/// starts, idle between turns, blocked on a permission, interrupted after
/// a cancelled turn until the next one, exited once it has ended and the
/// archive once archived (ADR 0026), with its end as its last activity.
/// Unread follows the records that ask for attention and the marks the
/// node hands over (protocol 10).
#[tokio::test]
async fn the_board_follows_turns_and_permissions() {
    within(async {
        let fixture = Fixture::new();
        let created = fixture.runtime.create(structured(None)).await.unwrap();
        let stored = collect(created.durable);
        let handle = created.handle;
        let mut watch = handle.watch_info();

        // The handshake done, no turn yet: between turns. No node sees
        // pull requests, so not "awaiting PR".
        let ready = board_until(&mut watch, DisplayStatus::Idle).await;
        assert_eq!(
            (ready.column, ready.activity),
            (KanbanColumn::Building, Activity::Idle)
        );
        assert!(!handle.info().marks.unread, "nothing asked yet");

        // The default prompt asks a permission and waits for the answer.
        handle.send(prompt("hello")).await.unwrap();
        let asked = board_until(&mut watch, DisplayStatus::Blocked).await;
        assert_eq!(asked.activity, Activity::WaitingInput);
        assert!(asked.last_activity_at > ready.last_activity_at);
        assert!(
            handle.info().marks.unread,
            "a permission asks for attention"
        );
        // Read through everything published: seen.
        let read = KeptMarks {
            read: Some(handle.log_end()),
            ..KeptMarks::default()
        };
        handle.set_marks(read);
        assert!(!handle.info().marks.unread, "read");
        let request = handle
            .follow(vessel_proto::EventSeq::ZERO)
            .unwrap()
            .cached
            .iter()
            .find_map(|record| {
                if let SessionEvent::Agent {
                    event: AgentEvent::PermissionRequested(request),
                } = &record.event
                {
                    Some(request.id)
                } else {
                    None
                }
            })
            .unwrap();
        handle
            .send(SessionInput::AnswerPermission {
                id: request,
                outcome: PermissionOutcome::Selected {
                    option: PermissionOptionId::new("allow-once"),
                },
            })
            .await
            .unwrap();
        let done = board_until(&mut watch, DisplayStatus::Idle).await;
        assert_eq!(done.activity, Activity::Idle);
        assert!(handle.info().marks.unread, "the turn's end, after the read");

        // A turn stopped before its end is interrupted ...
        handle.send(prompt("wait")).await.unwrap();
        let working = board_until(&mut watch, DisplayStatus::Working).await;
        assert_eq!(working.activity, Activity::Active);
        handle
            .send(SessionInput::CancelTurn { turn: None })
            .await
            .unwrap();
        let stopped = board_until(&mut watch, DisplayStatus::Interrupted).await;
        assert_eq!(
            (stopped.column, stopped.activity),
            (KanbanColumn::Building, Activity::Idle)
        );
        // ... until the next turn.
        handle.send(prompt("plain")).await.unwrap();
        assert_eq!(
            board_until(&mut watch, DisplayStatus::Idle).await.activity,
            Activity::Idle
        );

        handle.kill();
        let end = handle.ended().await;
        // ADR 0026: ended, and in building as exited until it is archived.
        assert_eq!(
            end.board,
            Some(SessionBoard::new(
                KanbanColumn::Building,
                DisplayStatus::Exited,
                Activity::Exited,
                end.ended_at.unwrap(),
            ))
        );
        // The node archives it in the store and hands the marks over; its
        // info then places it in the archive and says so.
        let archived_at = end.ended_at.unwrap();
        handle.set_marks(KeptMarks {
            archived_at: Some(archived_at),
            ..KeptMarks::default()
        });
        let put_away = handle.info();
        assert_eq!(put_away.board, Some(SessionBoard::archived(archived_at)));
        assert_eq!(put_away.marks.archived_at, Some(archived_at));
        drop(stored.await.unwrap());
    })
    .await;
}

/// A session that never started is in building as exited too: whatever
/// its status, an ended session is, until it is archived.
#[tokio::test]
async fn a_failed_session_is_exited() {
    within(async {
        let fixture = Fixture::new();
        let created = fixture
            .runtime
            .create(structured(Some("gpt-imaginary")))
            .await
            .unwrap();
        let stored = collect(created.durable);
        let end = created.handle.ended().await;
        assert!(matches!(end.status, SessionStatus::Failed { .. }));
        let board = end.board.unwrap();
        assert_eq!(
            (board.column, board.display_status, board.activity),
            (
                KanbanColumn::Building,
                DisplayStatus::Exited,
                Activity::Exited
            )
        );
        assert_eq!(Some(board.last_activity_at), end.ended_at);
        drop(stored.await.unwrap());
    })
    .await;
}

/// A session still starting (its adapter never answers) is working: on
/// the board from its creation, its last activity its creation.
#[tokio::test]
async fn a_starting_session_is_working() {
    within(Box::pin(async {
        let fixture = Fixture::with_stuck_adapter();
        let created = fixture.runtime.create(structured(None)).await.unwrap();
        let stored = collect(created.durable);
        let info = created.handle.info();
        assert_eq!(
            info.board,
            Some(SessionBoard::new(
                KanbanColumn::Building,
                DisplayStatus::Working,
                Activity::Active,
                info.created_at,
            ))
        );
        created.handle.kill();
        let end = created.handle.ended().await;
        assert_eq!(
            end.board.map(|board| (board.column, board.display_status)),
            Some((KanbanColumn::Building, DisplayStatus::Exited))
        );
        drop(stored.await.unwrap());
    }))
    .await;
}
