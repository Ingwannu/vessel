//! Ledger facts retain exact counts, field coverage and independent cost units.
use super::{TokenField, timestamp, token_count};
use crate::{grouped, localization::Translator};
use chrono::FixedOffset;
use vessel_proto::usage::{
    CostVisibility, OptionalUsageReports, ReportCount, TokenAccounting, UsageReport, UsageTotals,
};

/// Presentation prose contains no hidden identity or accounting arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportFact {
    /// Translated whole row label.
    pub label: String,
    /// Exact value or its explicit absence/visibility state.
    pub value: String,
    /// Coverage and attribution constraints.
    pub detail: Option<String>,
}

/// Each optional field retains its own coverage; populated is not complete.
#[must_use]
pub fn total_facts(
    totals: &UsageTotals,
    visibility: CostVisibility,
    words: &'static Translator,
) -> Vec<ReportFact> {
    let mut facts = token_facts(totals, words);
    facts.extend(cost_facts(totals, visibility, words));
    if totals.unaccounted_costs.get() > 0 {
        facts.push(ReportFact {
            label: words.usage().unaccounted_costs().to_owned(),
            value: grouped(u64::from(totals.unaccounted_costs.get())),
            detail: Some(words.usage().unaccounted_hint().to_owned()),
        });
    }
    facts
}
fn token_facts(totals: &UsageTotals, words: &'static Translator) -> Vec<ReportFact> {
    // Widening before addition preserves both wire counters without u32 saturation.
    let turns = u64::from(totals.reported.get()).saturating_add(u64::from(totals.unreported.get()));
    let mut facts: Vec<_> = TokenField::ALL
        .into_iter()
        .map(|field| {
            let reported = optional_count(field, &totals.optional_reports, totals.reported);
            let detail = words
                .usage()
                .turn_coverage(&grouped(u64::from(reported.get())), &grouped(turns))
                .ok();
            ReportFact {
                label: field.label(words).to_owned(),
                value: token_count(field.count(totals.tokens.as_ref()), words),
                detail,
            }
        })
        .collect();
    facts.push(ReportFact {
        label: words.usage().unreported().to_owned(),
        value: grouped(u64::from(totals.unreported.get())),
        detail: None,
    });
    facts
}
fn cost_facts(
    totals: &UsageTotals,
    visibility: CostVisibility,
    words: &'static Translator,
) -> Vec<ReportFact> {
    let text = words.usage();
    match visibility {
        CostVisibility::Withheld => vec![ReportFact {
            label: text.cost().to_owned(),
            value: text.withheld().to_owned(),
            detail: Some(text.withheld_hint().to_owned()),
        }],
        CostVisibility::Permitted if totals.costs.is_empty() => vec![ReportFact {
            label: text.cost().to_owned(),
            value: text.none().to_owned(),
            detail: None,
        }],
        CostVisibility::Permitted => totals
            .costs
            .iter()
            .map(|cost| ReportFact {
                label: text
                    .cost_unit(cost.unit.as_str())
                    .unwrap_or_else(|_format| text.cost().to_owned()),
                value: cost.amount.get().to_string(),
                detail: Some(text.cost_hint().to_owned()),
            })
            .collect(),
    }
}
fn optional_count(
    field: TokenField,
    optional: &OptionalUsageReports,
    reported: ReportCount,
) -> ReportCount {
    match field {
        TokenField::Total | TokenField::Input | TokenField::Output => reported,
        TokenField::Thought => optional.thought,
        TokenField::CachedRead => optional.cached_read,
        TokenField::CachedWrite => optional.cached_write,
    }
}

/// Observation and retention describe the actual evidence, never inferred history.
#[must_use]
pub fn coverage_facts(
    report: &UsageReport,
    offset: FixedOffset,
    words: &'static Translator,
) -> Vec<ReportFact> {
    let text = words.usage();
    [
        (text.observed(), report.at, None),
        (
            text.accounting_started(),
            report.accounting_started,
            Some(text.no_backfill().to_owned()),
        ),
        (text.retained_from(), report.retained_from, None),
    ]
    .into_iter()
    .map(|(label, at, detail)| ReportFact {
        label: label.to_owned(),
        value: timestamp(at, offset, words),
        detail,
    })
    .collect()
}

/// Overflow and retention gaps remain explicit even when displayed values fit.
#[must_use]
pub fn partial_report(report: &UsageReport, words: &'static Translator) -> Option<&'static str> {
    match report.totals.token_accounting {
        TokenAccounting::Overflow => Some(words.usage().overflow()),
        TokenAccounting::Reported => report.incomplete.then(|| words.usage().partial_hint()),
    }
}
#[cfg(test)]
mod tests;
