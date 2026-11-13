//! Temporal attribution and native-session continuity fixtures.
use super::tests::{cost, db, report};
use super::*;
use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageScope};
use vessel_proto::{AgentEvent, EventSeq, HarnessSessionId, SessionEvent, SessionStatus};

#[test]
fn a_period_cutting_an_observation_interval_does_not_take_all_its_money() {
    let (db, session, now) = db();
    let old = now - chrono::TimeDelta::hours(26);
    let created = old - chrono::TimeDelta::hours(1);
    let _rows = db
        .execute(
            "UPDATE sessions SET created_at = ?1 WHERE id = ?2",
            params![time_to_sql(created).unwrap(), session.to_string()],
        )
        .unwrap();
    let _rows = db
        .execute(
            "UPDATE usage_meta SET started_at = ?1",
            [time_to_sql(created).unwrap()],
        )
        .unwrap();
    let _appended = log::append(
        &db,
        session,
        &[
            EventRecord {
                seq: EventSeq::ZERO,
                at: old,
                event: cost(10.0),
            },
            EventRecord {
                seq: EventSeq::new(1),
                at: now,
                event: cost(12.0),
            },
        ],
    )
    .unwrap();
    let day = query(
        &db,
        &QueryUsage {
            scope: UsageScope::Session { session },
            period: UsagePeriod::Day,
            after: None,
        },
        now,
    )
    .unwrap();
    assert!(day.totals.costs.is_empty());
    assert_eq!(day.totals.unaccounted_costs.get(), 1);
    assert_eq!(
        day.current_costs[0].amount,
        FiniteNumber::new(12.0).unwrap()
    );
    let week = query(
        &db,
        &QueryUsage {
            scope: UsageScope::Session { session },
            period: UsagePeriod::Week,
            after: None,
        },
        now,
    )
    .unwrap();
    assert_eq!(
        week.totals.costs[0].amount,
        FiniteNumber::new(12.0).unwrap()
    );
    assert_eq!(week.totals.unaccounted_costs.get(), 0);
}
#[test]
fn replayed_cost_before_same_id_ready_is_not_spent_twice() {
    let (db, session, now) = db();
    let events = [
        cost(10.0),
        SessionEvent::Status {
            status: SessionStatus::Starting,
        },
        cost(10.0),
        SessionEvent::Agent {
            event: AgentEvent::Ready {
                harness_session: HarnessSessionId::new("fixture"),
                abilities: None,
            },
        },
        cost(10.0),
        cost(12.0),
    ];
    let records: Vec<_> = events
        .into_iter()
        .enumerate()
        .map(|(seq, event)| EventRecord {
            seq: EventSeq::new(u64::try_from(seq).unwrap()),
            at: now,
            event,
        })
        .collect();
    let _appended = log::append(&db, session, &records).unwrap();
    assert_eq!(
        report(&db, session, now).totals.costs[0].amount,
        FiniteNumber::new(12.0).unwrap()
    );
}
use crate::log;

#[test]
fn same_native_session_ready_preserves_cost_and_a_new_identity_is_unknown() {
    let (db, session, now) = db();
    let ready = |id| SessionEvent::Agent {
        event: AgentEvent::Ready {
            harness_session: HarnessSessionId::new(id),
            abilities: None,
        },
    };
    let events = [
        cost(10.0),
        ready("fixture"),
        cost(10.0),
        cost(12.0),
        ready("different"),
        cost(50.0),
        cost(52.0),
    ];
    let records: Vec<_> = events
        .into_iter()
        .enumerate()
        .map(|(seq, event)| EventRecord {
            seq: EventSeq::new(u64::try_from(seq).unwrap()),
            at: now,
            event,
        })
        .collect();
    let _appended = log::append(&db, session, &records[..4]).unwrap();
    assert_eq!(
        report(&db, session, now).totals.costs[0].amount,
        FiniteNumber::new(12.0).unwrap()
    );
    let _appended = log::append(&db, session, &records[4..]).unwrap();
    let report = report(&db, session, now);
    assert_eq!(
        report.totals.costs[0].amount,
        FiniteNumber::new(14.0).unwrap()
    );
    assert_eq!(report.totals.unaccounted_costs.get(), 1);
    assert_eq!(
        report.current_costs[0].amount,
        FiniteNumber::new(52.0).unwrap()
    );
}

#[test]
fn existing_sessions_first_cumulative_cost_is_not_today_spend() {
    let (db, session, now) = db();
    let _rows = db
        .execute(
            "UPDATE sessions SET usage_cost_baseline = NULL WHERE id = ?1",
            [session.to_string()],
        )
        .unwrap();
    cost::ready(&db, session, &HarnessSessionId::new("existing")).unwrap();
    let _appended = log::append(
        &db,
        session,
        &[EventRecord {
            seq: EventSeq::ZERO,
            at: now,
            event: cost(10.0),
        }],
    )
    .unwrap();
    let report = report(&db, session, now);
    assert!(report.totals.costs.is_empty());
    assert_eq!(report.totals.unaccounted_costs.get(), 1);
    assert_eq!(
        report.current_costs[0].amount,
        FiniteNumber::new(10.0).unwrap()
    );
}

#[test]
fn a_remote_new_session_is_fresh_but_an_import_has_unknown_prior_cost() {
    let (db, _session, now) = db();
    for (origin, fresh) in [
        (
            vessel_proto::SessionOrigin::Remote {
                node: vessel_proto::NodeId::random(),
                by: vessel_proto::RemoteCreator::Owner,
            },
            true,
        ),
        (
            vessel_proto::SessionOrigin::Imported {
                node: vessel_proto::NodeId::random(),
                session: SessionId::random(),
            },
            false,
        ),
    ] {
        let id = SessionId::random();
        let mut info = vessel_proto::SessionInfo::new(
            id,
            SessionSpec::new(
                vessel_proto::HarnessKind::ClaudeCode,
                vessel_proto::SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            ),
            now,
        );
        info.origin = Some(origin);
        log::create_session(&db, &info).unwrap();
        cost::ready(&db, id, &HarnessSessionId::new("native")).unwrap();
        let _appended = log::append(
            &db,
            id,
            &[EventRecord {
                seq: EventSeq::ZERO,
                at: now,
                event: cost(0.5),
            }],
        )
        .unwrap();
        let report = report(&db, id, now);
        assert_eq!(!report.totals.costs.is_empty(), fresh);
        assert_eq!(report.totals.unaccounted_costs.get(), u32::from(!fresh));
    }
}

#[test]
fn row_retention_keeps_exactly_the_newest_even_when_times_tie() {
    let (db, session, now) = db();
    let records: Vec<_> = (0..5)
        .map(|seq| EventRecord {
            seq: EventSeq::new(seq),
            at: now,
            event: SessionEvent::Agent {
                event: AgentEvent::TurnEnded {
                    reason: vessel_proto::StopReason::EndTurn,
                    usage: None,
                },
            },
        })
        .collect();
    let _appended = log::append(&db, session, &records).unwrap();
    prune_to(&db, now, 3).unwrap();
    let report = report(&db, session, now);
    assert_eq!(
        report
            .samples
            .iter()
            .map(|sample| sample.seq.get())
            .collect::<Vec<_>>(),
        [2, 3, 4]
    );
    assert!(report.incomplete);
    let count: i64 = db
        .query_row("SELECT sample_count FROM usage_meta", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 3);
    let _rows = db
        .execute("DELETE FROM sessions WHERE id = ?1", [session.to_string()])
        .unwrap();
    let count: i64 = db
        .query_row("SELECT sample_count FROM usage_meta", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn a_legacy_last_report_is_visible_without_backfilling_today_spend() {
    let (db, session, now) = db();
    let _appended = log::append(
        &db,
        session,
        &[EventRecord {
            seq: EventSeq::ZERO,
            at: now,
            event: cost(10.0),
        }],
    )
    .unwrap();
    let _rows = db.execute("DELETE FROM usage_samples", []).unwrap();
    let _rows = db
        .execute(
            "UPDATE sessions SET usage_cost_baseline = NULL WHERE id = ?1",
            [session.to_string()],
        )
        .unwrap();
    let report = report(&db, session, now);
    assert!(report.totals.costs.is_empty());
    assert_eq!(
        report.current_costs[0].amount,
        FiniteNumber::new(10.0).unwrap()
    );
}
