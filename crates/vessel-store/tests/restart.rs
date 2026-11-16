//! Real `SQLite` recovery projection, retention, stop fences and consumed intent.
//! These shared projection fixtures remain together so the stop/rearm sequence
//! and retained candidate state can be compared directly; exclusions and
//! lifecycle-specific cases have their own child modules.
#![cfg(test)]
use fixtures::{at, data_dir, record, status};
use std::sync::Arc;
use vessel_proto::restart::{RestartCheckpoint, RestartGeneration};
use vessel_proto::{
    AgentEvent, ContentBlock, EventSeq, HarnessKind, SessionEvent, SessionId, SessionInfo,
    SessionMode, SessionSpec, SessionStatus, StopReason,
};
use vessel_store::{ByteCount, RestartEligibility, Retention, Store, StoreConfig};
// SQL-only cases previously had no finite safety net. Preparation is kept
// outside this clock by the shared testkit, just like process-level cases.
const STORE_CASE_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);
const STORE_PHASE_BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

async fn phase<F: Future>(name: &str, future: F) -> F::Output {
    assert!(STORE_PHASE_BUDGET < STORE_CASE_BUDGET);
    tokio::time::timeout(STORE_PHASE_BUDGET, future)
        .await
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

async fn case<F: Future<Output = ()>>(future: F) {
    vessel_testkit::within(STORE_CASE_BUDGET, Box::pin(future))
        .await
        .unwrap();
}

#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "shared fixture builders include registry rollback used only by legacy migration tests"
)]
mod fixtures;

fn info() -> SessionInfo {
    SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        at(0),
    )
}
fn started(seq: u64) -> Arc<vessel_proto::EventRecord> {
    record(
        seq,
        SessionEvent::Agent {
            event: AgentEvent::TurnStarted {
                content: vec![ContentBlock::Text {
                    text: "Keep working".to_owned(),
                }],
            },
        },
    )
}
async fn working(store: &Store) -> SessionInfo {
    let session = info();
    phase(
        "Store create_session_with_restart",
        store.create_session_with_restart(session.clone(), RestartEligibility::Person),
    )
    .await
    .unwrap();
    phase(
        "Store append",
        store.append(
            session.id,
            vec![status(0, SessionStatus::Running), started(1)],
        ),
    )
    .await
    .unwrap();
    session
}
async fn snapshot(store: &Store, session: SessionId) {
    assert!(
        phase(
            "Store checkpoint_restart",
            store.checkpoint_restart(
                session,
                Vec::new(),
                RestartCheckpoint::Shutdown {
                    generation: RestartGeneration::ZERO,
                    turn: Some(EventSeq::new(1)),
                    parked: false,
                }
            )
        )
        .await
        .unwrap()
    );
}

#[tokio::test]
async fn committed_active_turn_survives_retention_and_startup_status_normalization() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig {
            retention: Retention {
                max_session_bytes: ByteCount::new(1024),
                ..Retention::default()
            },
            ..StoreConfig::default()
        };
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        let session = phase("Store working", working(&store)).await;
        let logs = (2..150)
            .map(|seq| {
                record(
                    seq,
                    SessionEvent::Agent {
                        event: AgentEvent::HarnessLog {
                            line: "tool output".repeat(40),
                        },
                    },
                )
            })
            .collect();
        phase("Store append", store.append(session.id, logs))
            .await
            .unwrap();
        assert!(
            phase(
                "Store read_events",
                store.read_events(session.id, EventSeq::ZERO, 10)
            )
            .await
            .unwrap()
            .records
            .iter()
            .all(|record| record.seq > EventSeq::new(1))
        );
        phase("Store close", store.close()).await;
        let opened = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap();
        assert_eq!(opened.interrupted.len(), 1);
        let candidate = phase("Store next_restart", opened.store.next_restart())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(candidate.session, session.id);
        assert_eq!(candidate.turn, EventSeq::new(1));
        assert_eq!(
            phase("Store session", opened.store.session(session.id))
                .await
                .unwrap()
                .unwrap()
                .status,
            SessionStatus::Interrupted
        );
        phase("Store close", opened.store.close()).await;
    })
    .await;
}

#[tokio::test]
async fn clean_shutdown_capture_survives_the_cancelled_turn_and_final_status() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig::default();
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        let session = phase("Store working", working(&store)).await;
        phase("Store snapshot", snapshot(&store, session.id)).await;
        phase(
            "Store append",
            store.append(
                session.id,
                vec![
                    record(
                        2,
                        SessionEvent::Agent {
                            event: AgentEvent::TurnEnded {
                                reason: StopReason::Cancelled,
                                usage: None,
                                point: None,
                            },
                        },
                    ),
                    status(3, SessionStatus::Interrupted),
                ],
            ),
        )
        .await
        .unwrap();
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap()
                .unwrap()
                .session,
            session.id
        );
        phase("Store close", store.close()).await;
    })
    .await;
}

// Keep the generation/cursor choices in the scenario while naming the real
// writer boundary once. These helpers return the actual Store acknowledgement.
async fn arm(
    store: &Store,
    session: SessionId,
    generation: RestartGeneration,
    turn: EventSeq,
) -> bool {
    checkpoint(store, session, RestartCheckpoint::Arm { generation, turn }).await
}

async fn suppress(
    store: &Store,
    session: SessionId,
    generation: RestartGeneration,
    turn: Option<EventSeq>,
) -> bool {
    checkpoint(
        store,
        session,
        RestartCheckpoint::Suppress { generation, turn },
    )
    .await
}

async fn checkpoint(store: &Store, session: SessionId, change: RestartCheckpoint) -> bool {
    phase(
        "Store checkpoint_restart",
        store.checkpoint_restart(session, Vec::new(), change),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn stop_generation_rejects_late_start_and_ack_but_accepts_a_new_turn() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig::default();
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        let session = phase("Store working", working(&store)).await;
        let next = RestartGeneration::ZERO.next().unwrap();
        assert!(suppress(&store, session.id, next, Some(EventSeq::new(1))).await);
        phase("Store append", store.append(session.id, vec![started(2)]))
            .await
            .unwrap();
        assert!(
            !arm(
                &store,
                session.id,
                RestartGeneration::ZERO,
                EventSeq::new(2)
            )
            .await
        );
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap(),
            None
        );
        let ended = phase("Store session", store.session(session.id))
            .await
            .unwrap()
            .unwrap();
        let _marks = phase("Store reopen", store.reopen(session.id, ended.next_seq))
            .await
            .unwrap();
        // Reopen enters Starting. A real actor publishes Running before a new
        // turn; Starting alone must never qualify for automatic startup work.
        let seq = ended.next_seq.next().unwrap();
        phase(
            "Store append",
            store.append(
                session.id,
                vec![
                    status(ended.next_seq.get(), SessionStatus::Running),
                    record(
                        seq.get(),
                        SessionEvent::Agent {
                            event: AgentEvent::TurnStarted {
                                content: vec![ContentBlock::Text {
                                    text: "New explicit input".to_owned(),
                                }],
                            },
                        },
                    ),
                ],
            ),
        )
        .await
        .unwrap();
        assert!(arm(&store, session.id, next, seq).await);
        assert!(
            !suppress(
                &store,
                session.id,
                next.next().unwrap(),
                Some(EventSeq::new(1))
            )
            .await
        );
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap()
                .unwrap()
                .turn,
            seq
        );
        phase("Store close", store.close()).await;
    })
    .await;
}

#[tokio::test]
async fn disabled_startup_consumes_intent_and_a_consumed_claim_is_not_retried() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig::default();
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        let session = phase("Store working", working(&store)).await;
        phase("Store snapshot", snapshot(&store, session.id)).await;
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, false),
        )
        .await
        .unwrap()
        .store;
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap(),
            None
        );
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap(),
            None
        );
        let other = phase("Store working", working(&store)).await;
        phase("Store snapshot", snapshot(&store, other.id)).await;
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        let candidate = phase("Store next_restart", store.next_restart())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(candidate.session, other.id);
        assert!(
            phase("Store claim_restart", store.claim_restart(candidate))
                .await
                .unwrap()
        );
        assert!(
            !phase("Store claim_restart", store.claim_restart(candidate))
                .await
                .unwrap()
        );
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        // The consumed launch had not started another committed turn.
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap(),
            None
        );
        phase("Store close", store.close()).await;
    })
    .await;
}

#[tokio::test]
async fn pending_and_claimed_rows_are_protected_until_the_attempt_settles() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig {
            retention: Retention {
                max_ended_sessions: 0,
                ..Retention::default()
            },
            ..StoreConfig::default()
        };
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        let session = phase("Store working", working(&store)).await;
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        let candidate = phase("Store next_restart", store.next_restart())
            .await
            .unwrap()
            .unwrap();
        assert!(
            phase("Store claim_restart", store.claim_restart(candidate))
                .await
                .unwrap()
        );
        let other = info();
        phase("Store create_session", store.create_session(other.clone()))
            .await
            .unwrap();
        phase(
            "Store append",
            store.append(
                other.id,
                vec![status(0, SessionStatus::Exited { code: Some(0) })],
            ),
        )
        .await
        .unwrap();
        assert!(
            phase("Store session", store.session(session.id))
                .await
                .unwrap()
                .is_some()
        );
        phase("Store finish_restart", store.finish_restart(session.id))
            .await
            .unwrap();
        assert_eq!(
            phase("Store session", store.session(session.id))
                .await
                .unwrap(),
            None
        );
        phase("Store close", store.close()).await;
    })
    .await;
}

#[tokio::test]
async fn checkpoints_do_not_add_public_records_and_terminal_output_is_not_an_active_turn() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig::default();
        let store = phase("Store open", fixtures::open(&dir, config)).await;
        let session = fixtures::new_session(0);
        phase(
            "Store create_session_with_restart",
            store.create_session_with_restart(session.clone(), RestartEligibility::Person),
        )
        .await
        .unwrap();
        let records = vec![fixtures::output(0, 32)];
        phase("Store append", store.append(session.id, records.clone()))
            .await
            .unwrap();
        assert!(
            phase(
                "Store checkpoint_restart",
                store.checkpoint_restart(
                    session.id,
                    Vec::new(),
                    RestartCheckpoint::Suppress {
                        generation: RestartGeneration::ZERO.next().unwrap(),
                        turn: None,
                    }
                )
            )
            .await
            .unwrap()
        );
        let page = phase(
            "Store read_events",
            store.read_events(session.id, EventSeq::ZERO, 10),
        )
        .await
        .unwrap();
        assert_eq!(page.records, fixtures::plain(&records));
        assert_eq!(page.next, EventSeq::new(1));
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, config, true),
        )
        .await
        .unwrap()
        .store;
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap(),
            None
        );
        phase("Store close", store.close()).await;
    })
    .await;
}
#[path = "restart/lifecycle.rs"]
mod lifecycle;

#[path = "restart/exclusions.rs"]
mod exclusions;

#[path = "restart/attention.rs"]
mod attention;

#[path = "restart/creation.rs"]
mod creation;
