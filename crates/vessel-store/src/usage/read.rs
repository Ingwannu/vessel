//! One indexed, bounded snapshot for both totals and its session page.

use super::{DAYS_KEPT, MAX_SAMPLES, totals};
use crate::codec::{decode, encode, session_id_from_sql, time_from_sql, time_to_sql};
use crate::error::{Column, StoreError};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::usage::{
    QueryUsage, UsagePeriod, UsageReport, UsageSample, UsageScope, UsageTotals,
};
use vessel_proto::{HarnessRef, ProfileName};

pub(crate) fn query(
    connection: &Connection,
    request: &QueryUsage,
    now: DateTime<Utc>,
) -> Result<UsageReport, StoreError> {
    let transaction = connection.unchecked_transaction()?;
    let (started, pruned): (i64, Option<i64>) = transaction.query_row(
        "SELECT started_at, pruned_through FROM usage_meta",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let retained_from = super::before(now, DAYS_KEPT);
    let from = match request.period {
        UsagePeriod::Day => super::before(now, 1),
        UsagePeriod::Week => super::before(now, 7),
        UsagePeriod::Session => retained_from,
    };
    let (session, harness, profile) = match &request.scope {
        UsageScope::Machine => (None, None, None),
        UsageScope::Agent { harness, profile } => (
            None,
            Some(encode(harness)?),
            profile.as_ref().map(ProfileName::as_str),
        ),
        UsageScope::Session { session } => (Some(session.to_string()), None, None),
    };
    let mut statement = transaction.prepare_cached(
        "SELECT session_id, harness, profile, sample FROM usage_samples INDEXED BY usage_time
         WHERE at >= ?1 AND at <= ?2 AND (?3 IS NULL OR session_id = ?3)
         AND (?4 IS NULL OR harness = ?4) AND (?5 IS NULL OR profile = ?5)
         ORDER BY at, session_id, seq LIMIT ?6",
    )?;
    let mut rows = statement.query(params![
        time_to_sql(from)?,
        time_to_sql(now)?,
        session,
        harness,
        profile,
        MAX_SAMPLES
    ])?;
    let mut report = UsageReport {
        at: now,
        accounting_started: time_from_sql(started),
        retained_from,
        incomplete: pruned.is_some_and(|at| at >= time_to_sql(from).unwrap_or(i64::MIN)),
        current_costs: match request.scope {
            UsageScope::Session { session } => super::cost::current(&transaction, session)?,
            UsageScope::Agent { .. } | UsageScope::Machine => Vec::new(),
        },
        cost_visibility: vessel_proto::usage::CostVisibility::Permitted,
        totals: UsageTotals::default(),
        agents: Vec::new(),
        samples: Vec::new(),
        next: None,
    };
    let mut more_samples = false;
    while let Some(row) = rows.next()? {
        let id = session_id_from_sql(&row.get::<_, String>(0)?)?;
        let harness: HarnessRef = decode(id, Column::Spec, &row.get::<_, String>(1)?)?;
        let profile_text: String = row.get(2)?;
        let profile: Option<ProfileName> = if profile_text.is_empty() {
            None
        } else {
            Some(decode(id, Column::Profile, &encode(&profile_text)?)?)
        };
        let sample: UsageSample = decode(id, Column::Usage, &row.get::<_, String>(3)?)?;
        totals::add_selected(&mut report, &sample, harness, profile, from);
        if session.is_some() && request.after.is_none_or(|after| sample.seq > after) {
            let at = report
                .samples
                .binary_search_by_key(&sample.seq, |sample| sample.seq)
                .unwrap_or_else(|at| at);
            if at < 100 {
                report.samples.insert(at, sample);
            } else {
                more_samples = true;
            }
            if report.samples.len() > 100 {
                drop(report.samples.pop());
                more_samples = true;
            }
        }
    }
    // Sequence, not wall clock, is the only stable session cursor. Keep
    // just the lowest 100 matching sequences even if time moves backwards.
    if more_samples {
        report.next = report.samples.last().map(|sample| sample.seq);
    }
    report.agents.sort_by(|a, b| {
        a.harness
            .to_string()
            .cmp(&b.harness.to_string())
            .then_with(|| a.profile.cmp(&b.profile))
    });
    Ok(report)
}
