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

async fn case<F: std::future::Future<Output = ()>>(future: F) {
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
    store
        .create_session_with_restart(session.clone(), RestartEligibility::Person)
        .await
        .unwrap();
    store
        .append(
            session.id,
            vec![status(0, SessionStatus::Running), started(1)],
        )
        .await
        .unwrap();
    session
}
async fn snapshot(store: &Store, session: SessionId) {
    assert!(
        store
            .checkpoint_restart(
                session,
                Vec::new(),
                RestartCheckpoint::Shutdown {
                    generation: RestartGeneration::ZERO,
                    turn: Some(EventSeq::new(1)),
                    parked: false,
                }
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
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        let session = working(&store).await;
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
        store.append(session.id, logs).await.unwrap();
        assert!(
            store
                .read_events(session.id, EventSeq::ZERO, 10)
                .await
                .unwrap()
                .records
                .iter()
                .all(|record| record.seq > EventSeq::new(1))
        );
        store.close().await;
        let opened = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap();
        assert_eq!(opened.interrupted.len(), 1);
        let candidate = opened.store.next_restart().await.unwrap().unwrap();
        assert_eq!(candidate.session, session.id);
        assert_eq!(candidate.turn, EventSeq::new(1));
        assert_eq!(
            opened
                .store
                .session(session.id)
                .await
                .unwrap()
                .unwrap()
                .status,
            SessionStatus::Interrupted
        );
        opened.store.close().await;
    })
    .await;
}

#[tokio::test]
async fn clean_shutdown_capture_survives_the_cancelled_turn_and_final_status() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig::default();
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        let session = working(&store).await;
        snapshot(&store, session.id).await;
        store
            .append(
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
            )
            .await
            .unwrap();
        store.close().await;
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        assert_eq!(
            store.next_restart().await.unwrap().unwrap().session,
            session.id
        );
        store.close().await;
    })
    .await;
}

#[tokio::test]
async fn stop_generation_rejects_late_start_and_ack_but_accepts_a_new_turn() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig::default();
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        let session = working(&store).await;
        let next = RestartGeneration::ZERO.next().unwrap();
        assert!(
            store
                .checkpoint_restart(
                    session.id,
                    Vec::new(),
                    RestartCheckpoint::Suppress {
                        generation: next,
                        turn: Some(EventSeq::new(1))
                    }
                )
                .await
                .unwrap()
        );
        store.append(session.id, vec![started(2)]).await.unwrap();
        assert!(
            !store
                .checkpoint_restart(
                    session.id,
                    Vec::new(),
                    RestartCheckpoint::Arm {
                        generation: RestartGeneration::ZERO,
                        turn: EventSeq::new(2)
                    }
                )
                .await
                .unwrap()
        );
        store.close().await;
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        assert_eq!(store.next_restart().await.unwrap(), None);
        let ended = store.session(session.id).await.unwrap().unwrap();
        let _marks = store.reopen(session.id, ended.next_seq).await.unwrap();
        // Reopen enters Starting. A real actor publishes Running before a new
        // turn; Starting alone must never qualify for automatic startup work.
        let seq = ended.next_seq.next().unwrap();
        store
            .append(
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
            )
            .await
            .unwrap();
        assert!(
            store
                .checkpoint_restart(
                    session.id,
                    Vec::new(),
                    RestartCheckpoint::Arm {
                        generation: next,
                        turn: seq
                    }
                )
                .await
                .unwrap()
        );
        assert!(
            !store
                .checkpoint_restart(
                    session.id,
                    Vec::new(),
                    RestartCheckpoint::Suppress {
                        generation: next.next().unwrap(),
                        turn: Some(EventSeq::new(1))
                    }
                )
                .await
                .unwrap()
        );
        store.close().await;
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        assert_eq!(store.next_restart().await.unwrap().unwrap().turn, seq);
        store.close().await;
    })
    .await;
}

#[tokio::test]
async fn disabled_startup_consumes_intent_and_a_consumed_claim_is_not_retried() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig::default();
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        let session = working(&store).await;
        snapshot(&store, session.id).await;
        store.close().await;
        let store = Store::open_with_restart_policy(&dir, config, false)
            .await
            .unwrap()
            .store;
        assert_eq!(store.next_restart().await.unwrap(), None);
        store.close().await;
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        assert_eq!(store.next_restart().await.unwrap(), None);
        let other = working(&store).await;
        snapshot(&store, other.id).await;
        store.close().await;
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        let candidate = store.next_restart().await.unwrap().unwrap();
        assert_eq!(candidate.session, other.id);
        assert!(store.claim_restart(candidate).await.unwrap());
        assert!(!store.claim_restart(candidate).await.unwrap());
        store.close().await;
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        // The consumed launch had not started another committed turn.
        assert_eq!(store.next_restart().await.unwrap(), None);
        store.close().await;
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
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        let session = working(&store).await;
        store.close().await;
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        let candidate = store.next_restart().await.unwrap().unwrap();
        assert!(store.claim_restart(candidate).await.unwrap());
        let other = info();
        store.create_session(other.clone()).await.unwrap();
        store
            .append(
                other.id,
                vec![status(0, SessionStatus::Exited { code: Some(0) })],
            )
            .await
            .unwrap();
        assert!(store.session(session.id).await.unwrap().is_some());
        store.finish_restart(session.id).await.unwrap();
        assert_eq!(store.session(session.id).await.unwrap(), None);
        store.close().await;
    })
    .await;
}

#[tokio::test]
async fn checkpoints_do_not_add_public_records_and_terminal_output_is_not_an_active_turn() {
    case(async {
        let (_temp, dir) = data_dir();
        let config = StoreConfig::default();
        let store = fixtures::open(&dir, config).await;
        let session = fixtures::new_session(0);
        store
            .create_session_with_restart(session.clone(), RestartEligibility::Person)
            .await
            .unwrap();
        let records = vec![fixtures::output(0, 32)];
        store.append(session.id, records.clone()).await.unwrap();
        assert!(
            store
                .checkpoint_restart(
                    session.id,
                    Vec::new(),
                    RestartCheckpoint::Suppress {
                        generation: RestartGeneration::ZERO.next().unwrap(),
                        turn: None,
                    }
                )
                .await
                .unwrap()
        );
        let page = store
            .read_events(session.id, EventSeq::ZERO, 10)
            .await
            .unwrap();
        assert_eq!(page.records, fixtures::plain(&records));
        assert_eq!(page.next, EventSeq::new(1));
        store.close().await;
        let store = Store::open_with_restart_policy(&dir, config, true)
            .await
            .unwrap()
            .store;
        assert_eq!(store.next_restart().await.unwrap(), None);
        store.close().await;
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
