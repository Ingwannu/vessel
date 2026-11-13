use super::*;
use crate::{log, schema};
use vessel_proto::conversation::Cost;
use vessel_proto::conversation::FiniteNumber;
use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageScope};
use vessel_proto::{
    ContextUsage, EventSeq, HarnessKind, HarnessSessionId, SessionInfo, SessionMode, SessionSpec,
    StopReason, TokenCount, TurnUsage,
};

pub(super) fn db() -> (Connection, SessionId, DateTime<Utc>) {
    let mut db = Connection::open_in_memory().unwrap();
    schema::configure(&db).unwrap();
    schema::migrate(&mut db).unwrap();
    let now = Utc::now();
    let session = SessionId::random();
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    log::create_session(&db, &SessionInfo::new(session, spec, now)).unwrap();
    cost::ready(&db, session, &HarnessSessionId::new("fixture")).unwrap();
    (db, session, now)
}
pub(super) fn cost(value: f64) -> SessionEvent {
    SessionEvent::UsageChanged {
        usage: ContextUsage {
            used: TokenCount::new(1200),
            size: TokenCount::new(200_000),
            cost: Some(Cost {
                amount: FiniteNumber::new(value).unwrap(),
                unit: "USD".parse().unwrap(),
            }),
        },
    }
}
fn tokens() -> TurnUsage {
    TurnUsage {
        total: TokenCount::new(30),
        input: TokenCount::new(20),
        output: TokenCount::new(10),
        thought: None,
        cached_read: Some(TokenCount::new(12)),
        cached_write: None,
    }
}
pub(super) fn report(
    db: &Connection,
    session: SessionId,
    at: DateTime<Utc>,
) -> vessel_proto::usage::UsageReport {
    query(
        db,
        &QueryUsage {
            scope: UsageScope::Session { session },
            period: UsagePeriod::Session,
            after: None,
        },
        at,
    )
    .unwrap()
}
#[test]
fn running_cost_is_not_summed_and_turn_counts_stay_separate_from_context() {
    let (db, session, now) = db();
    let events = [
        cost(0.5),
        cost(0.5),
        cost(0.8),
        SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: Some(tokens()),
            },
        },
        SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: None,
            },
        },
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
    let report = report(&db, session, now);
    assert_eq!(report.totals.tokens, Some(tokens()));
    assert_eq!(report.totals.reported.get(), 1);
    assert_eq!(report.totals.unreported.get(), 1);
    assert_eq!(
        report.totals.costs[0].amount,
        FiniteNumber::new(0.8).unwrap()
    );
    assert_eq!(
        report.samples.len(),
        4,
        "repeated cumulative cost is not a new charge"
    );
}
#[test]
fn decreasing_cost_is_an_unknown_reset_and_ready_does_not_recharge() {
    let (db, session, now) = db();
    let events = [
        cost(1.0),
        cost(0.5),
        SessionEvent::Agent {
            event: AgentEvent::Ready {
                harness_session: HarnessSessionId::new("fixture"),
                abilities: None,
            },
        },
        cost(0.2),
        cost(-10.0),
        cost(0.3),
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
        FiniteNumber::new(1.1).unwrap()
    );
}
#[test]
fn rollback_pruning_pagination_and_session_delete_do_not_invent_usage() {
    let (mut db, session, now) = db();
    let record = |seq| EventRecord {
        seq: EventSeq::new(seq),
        at: now,
        event: SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: Some(tokens()),
            },
        },
    };
    {
        let transaction = db.transaction().unwrap();
        assert!(log::append(&transaction, session, &[record(0), record(9)]).is_err());
        transaction.rollback().unwrap();
    }
    assert_eq!(report(&db, session, now).totals.reported.get(), 0);
    let records: Vec<_> = (0..102).map(record).collect();
    let _appended = log::append(&db, session, &records).unwrap();
    let first = report(&db, session, now);
    assert_eq!(first.samples.len(), 100);
    assert_eq!(first.next, Some(EventSeq::new(99)));
    let second = query(
        &db,
        &QueryUsage {
            scope: UsageScope::Session { session },
            period: UsagePeriod::Session,
            after: first.next,
        },
        now,
    )
    .unwrap();
    assert_eq!(second.samples.len(), 2);
    assert_eq!(second.totals.reported.get(), 102);
    let _rows = db
        .execute(
            "DELETE FROM events WHERE session_id = ?1",
            [session.to_string()],
        )
        .unwrap();
    assert_eq!(report(&db, session, now).totals.reported.get(), 102);
    prune(&db, now + chrono::TimeDelta::days(91)).unwrap();
    assert_eq!(
        report(&db, session, now + chrono::TimeDelta::days(91))
            .totals
            .tokens,
        None
    );
    let _rows = db
        .execute("DELETE FROM sessions WHERE id = ?1", [session.to_string()])
        .unwrap();
    assert_eq!(report(&db, session, now).totals.tokens, None);
}

#[test]
fn profile_moves_in_one_batch_attribute_each_report_to_its_original_profile() {
    let (db, session, now) = db();
    let turn = || SessionEvent::Agent {
        event: AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: Some(tokens()),
        },
    };
    let events = [
        turn(),
        SessionEvent::ProfileChanged {
            from: None,
            to: Some("other".parse().unwrap()),
        },
        turn(),
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
    let report = report(&db, session, now);
    assert_eq!(report.agents.len(), 2);
    assert!(
        report
            .agents
            .iter()
            .all(|agent| agent.totals.reported.get() == 1)
    );
}

#[test]
fn a_reported_zero_is_distinct_from_no_report() {
    let (db, session, now) = db();
    assert_eq!(report(&db, session, now).totals.tokens, None);
    let mut zero = tokens();
    zero.total = TokenCount::new(0);
    zero.input = TokenCount::new(0);
    zero.output = TokenCount::new(0);
    zero.cached_read = Some(TokenCount::new(0));
    let _appended = log::append(
        &db,
        session,
        &[EventRecord {
            seq: EventSeq::ZERO,
            at: now,
            event: SessionEvent::Agent {
                event: AgentEvent::TurnEnded {
                    reason: StopReason::EndTurn,
                    usage: Some(zero),
                },
            },
        }],
    )
    .unwrap();
    assert_eq!(report(&db, session, now).totals.tokens, Some(zero));
}

#[test]
fn money_outside_decimal_range_and_token_overflow_are_explicit_gaps() {
    let (db, session, now) = db();
    let events = [cost(f64::MAX), cost(1.0)];
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
    let report = report(&db, session, now);
    assert_eq!(report.totals.unaccounted_costs.get(), 2);
    assert!(report.totals.costs.is_empty());
    assert_eq!(
        report.current_costs[0].amount,
        FiniteNumber::new(1.0).unwrap()
    );
    let mut total = totals::Total::default();
    let mut huge = tokens();
    huge.total = TokenCount::new(u64::MAX);
    total.add(&UsageSampleKind::Turn { usage: Some(huge) });
    total.add(&UsageSampleKind::Turn {
        usage: Some(tokens()),
    });
    let total = total.finish();
    assert_eq!(
        total.token_accounting,
        vessel_proto::usage::TokenAccounting::Overflow
    );
}

#[test]
fn zero_cost_and_alternating_currencies_keep_their_own_baselines() {
    let (db, session, now) = db();
    let mut credits = ContextUsage {
        used: TokenCount::new(0),
        size: TokenCount::new(200_000),
        cost: Some(Cost {
            amount: FiniteNumber::new(2.0).unwrap(),
            unit: "credits".parse().unwrap(),
        }),
    };
    credits.cost.as_mut().unwrap().unit = "credits".parse().unwrap();
    let events = [
        cost(0.0),
        cost(1.0),
        SessionEvent::UsageChanged { usage: credits },
        cost(1.5),
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
    let _appended = log::append(&db, session, &records[..1]).unwrap();
    assert_eq!(
        report(&db, session, now).totals.costs[0].amount,
        FiniteNumber::new(0.0).unwrap()
    );
    let _appended = log::append(&db, session, &records[1..]).unwrap();
    let report = report(&db, session, now);
    assert_eq!(
        report
            .totals
            .costs
            .iter()
            .find(|cost| cost.unit.as_str() == "USD")
            .unwrap()
            .amount,
        FiniteNumber::new(1.5).unwrap()
    );
    assert_eq!(
        report
            .totals
            .costs
            .iter()
            .find(|cost| cost.unit.as_str() == "credits")
            .unwrap()
            .amount,
        FiniteNumber::new(2.0).unwrap()
    );
}

#[test]
fn optional_token_fields_expose_their_missing_turns() {
    let mut total = totals::Total::default();
    let first = tokens();
    let mut second = tokens();
    second.cached_read = None;
    total.add(&UsageSampleKind::Turn { usage: Some(first) });
    total.add(&UsageSampleKind::Turn {
        usage: Some(second),
    });
    let total = total.finish();
    assert_eq!(total.tokens.unwrap().cached_read, Some(TokenCount::new(12)));
    assert_eq!(total.reported.get(), 2);
    assert_eq!(total.optional_reports.cached_read.get(), 1);
    assert_eq!(total.optional_reports.cached_write.get(), 0);
    assert_eq!(total.optional_reports.thought.get(), 0);
    assert_eq!(
        serde_json::to_string(&total).unwrap(),
        r#"{"tokens":{"total":60,"input":40,"output":20,"cached_read":12},"token_accounting":"reported","costs":[],"reported":2,"unreported":0,"unaccounted_costs":0,"optional_reports":{"thought":0,"cached_read":1,"cached_write":0}}"#
    );
}
