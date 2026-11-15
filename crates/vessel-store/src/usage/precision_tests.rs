//! Reports that used to round away at Decimal or finite-number boundaries.
use super::tests::{cost, db, report};
use super::*;
use crate::log;
use vessel_proto::conversation::Cost;
use vessel_proto::conversation::FiniteNumber;
use vessel_proto::usage::{QueryUsage, ReportCount, UsagePeriod, UsageScope, UsageTotals};
use vessel_proto::{EventSeq, HarnessKind, HarnessSessionId, SessionInfo, SessionMode};

#[test]
fn tiny_valid_finite_reports_are_explicitly_outside_the_decimal_domain() {
    let (db, session, now) = db();
    let _appended = log::append(
        &db,
        session,
        &[
            EventRecord {
                seq: EventSeq::ZERO,
                at: now,
                event: cost(1e-29),
            },
            EventRecord {
                seq: EventSeq::new(1),
                at: now,
                event: cost(2e-29),
            },
        ],
    )
    .unwrap();
    let value = report(&db, session, now);
    assert_eq!(
        value.totals,
        UsageTotals {
            unaccounted_costs: ReportCount::new(2),
            ..UsageTotals::default()
        }
    );
    assert_eq!(
        value
            .samples
            .iter()
            .map(|sample| &sample.kind)
            .collect::<Vec<_>>(),
        vec![
            &UsageSampleKind::CostUnavailable,
            &UsageSampleKind::CostUnavailable
        ]
    );
    assert_eq!(
        value.current_costs,
        vec![Cost {
            amount: FiniteNumber::new(2e-29).unwrap(),
            unit: "USD".parse().unwrap()
        }]
    );
}

#[test]
fn the_smallest_supported_decimal_value_remains_reported() {
    let value = machine(&[1e-28]);
    assert_cost(&value, Some(1e-28), 0);
}

#[test]
fn separate_small_costs_survive_a_large_sum_in_every_input_order() {
    for values in [
        [10_000_000_000_000_000.0, 1.0, 1.0],
        [1.0, 10_000_000_000_000_000.0, 1.0],
        [1.0, 1.0, 10_000_000_000_000_000.0],
    ] {
        assert_cost(&machine(&values), Some(10_000_000_000_000_002.0), 0);
    }
}

#[test]
fn an_unrepresentable_final_sum_is_absent_with_all_contributing_reports_counted() {
    for values in [
        [10_000_000_000_000_000.0, 1.0],
        [1.0, 10_000_000_000_000_000.0],
    ] {
        assert_cost(&machine(&values), None, 2);
    }
    for values in [[1e28, 1e-28], [1e-28, 1e28]] {
        assert_cost(&machine(&values), None, 2);
    }
}

#[test]
fn a_large_cumulative_increase_keeps_the_exact_increment() {
    let (db, session, now) = db();
    let _appended = log::append(
        &db,
        session,
        &[
            EventRecord {
                seq: EventSeq::ZERO,
                at: now,
                event: cost(10_000_000_000_000_000.0),
            },
            EventRecord {
                seq: EventSeq::new(1),
                at: now,
                event: cost(10_000_000_000_000_002.0),
            },
        ],
    )
    .unwrap();
    let value = report(&db, session, now);
    assert_cost(&value, Some(10_000_000_000_000_002.0), 0);
    assert_eq!(
        value.samples.last().unwrap().kind,
        UsageSampleKind::Cost {
            since: now,
            cost: Cost {
                amount: FiniteNumber::new(2.0).unwrap(),
                unit: "USD".parse().unwrap()
            }
        }
    );
}

fn assert_cost(report: &vessel_proto::usage::UsageReport, amount: Option<f64>, gaps: u32) {
    let expected = UsageTotals {
        costs: amount
            .into_iter()
            .map(|amount| Cost {
                amount: FiniteNumber::new(amount).unwrap(),
                unit: "USD".parse().unwrap(),
            })
            .collect(),
        unaccounted_costs: ReportCount::new(gaps),
        ..UsageTotals::default()
    };
    assert_eq!(report.totals, expected);
    assert_eq!(report.agents.len(), 1);
    assert_eq!(report.agents[0].totals, expected);
}

fn machine(values: &[f64]) -> vessel_proto::usage::UsageReport {
    let (db, _session, now) = db();
    for (position, value) in values.iter().enumerate() {
        let session = SessionId::random();
        log::create_session(
            &db,
            &SessionInfo::new(
                session,
                SessionSpec::new(
                    HarnessKind::ClaudeCode,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                ),
                now,
            ),
        )
        .unwrap();
        cost::ready(&db, session, &HarnessSessionId::new("fixture")).unwrap();
        let at = now
            .checked_add_signed(chrono::TimeDelta::seconds(i64::try_from(position).unwrap()))
            .unwrap();
        let _appended = log::append(
            &db,
            session,
            &[EventRecord {
                seq: EventSeq::ZERO,
                at,
                event: cost(*value),
            }],
        )
        .unwrap();
    }
    query(
        &db,
        &QueryUsage {
            scope: UsageScope::Machine,
            period: UsagePeriod::Day,
            after: None,
        },
        now.checked_add_signed(chrono::TimeDelta::minutes(1))
            .unwrap(),
    )
    .unwrap()
}
