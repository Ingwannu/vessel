//! Aggregation of reported fields. Cache values remain provider-defined;
//! they are never added again to input or total tokens.

use vessel_proto::conversation::{Cost, FiniteNumber};
use vessel_proto::usage::{TokenAccounting, UsageSampleKind, UsageTotals};
use vessel_proto::{TokenCount, TurnUsage};

pub(super) fn add(total: &mut UsageTotals, sample: &UsageSampleKind) {
    match sample {
        UsageSampleKind::CostUnavailable | UsageSampleKind::CostBaseline { .. } => {
            total.unaccounted_costs.increment();
        }
        UsageSampleKind::Turn { usage: None } => total.unreported.increment(),
        UsageSampleKind::Turn { usage: Some(usage) } => {
            total.reported.increment();
            if usage.thought.is_some() {
                total.optional_reports.thought.increment();
            }
            if usage.cached_read.is_some() {
                total.optional_reports.cached_read.increment();
            }
            if usage.cached_write.is_some() {
                total.optional_reports.cached_write.increment();
            }
            total.tokens = Some(match total.tokens {
                None => *usage,
                Some(previous) => TurnUsage {
                    total: sum(previous.total, usage.total, &mut total.token_accounting),
                    input: sum(previous.input, usage.input, &mut total.token_accounting),
                    output: sum(previous.output, usage.output, &mut total.token_accounting),
                    thought: optional(previous.thought, usage.thought, &mut total.token_accounting),
                    cached_read: optional(
                        previous.cached_read,
                        usage.cached_read,
                        &mut total.token_accounting,
                    ),
                    cached_write: optional(
                        previous.cached_write,
                        usage.cached_write,
                        &mut total.token_accounting,
                    ),
                },
            });
        }
        UsageSampleKind::Cost { cost, since: _ } => {
            if let Some(previous) = total
                .costs
                .iter_mut()
                .find(|previous| previous.unit == cost.unit)
            {
                let sum = super::decimal(previous.amount)
                    .zip(super::decimal(cost.amount))
                    .and_then(|(a, b)| a.checked_add(b))
                    .and_then(|value| FiniteNumber::new(value.as_f64()));
                if let Some(sum) = sum {
                    previous.amount = sum;
                } else {
                    total.unaccounted_costs.increment();
                }
            } else if total.costs.len() < 32 {
                total.costs.push(Cost {
                    amount: cost.amount,
                    unit: cost.unit.clone(),
                });
            } else {
                total.unaccounted_costs.increment();
            }
        }
    }
}
fn sum(a: TokenCount, b: TokenCount, accuracy: &mut TokenAccounting) -> TokenCount {
    if let Some(sum) = a.get().checked_add(b.get()) {
        TokenCount::new(sum)
    } else {
        *accuracy = TokenAccounting::Overflow;
        TokenCount::new(u64::MAX)
    }
}
fn optional(
    a: Option<TokenCount>,
    b: Option<TokenCount>,
    accuracy: &mut TokenAccounting,
) -> Option<TokenCount> {
    match (a, b) {
        (Some(a), Some(b)) => Some(sum(a, b, accuracy)),
        (a, b) => a.or(b),
    }
}

/// Group bounds keep a peer from receiving an arbitrarily large frame.
pub(super) fn add_agent(
    report: &mut vessel_proto::usage::UsageReport,
    harness: vessel_proto::HarnessRef,
    profile: Option<vessel_proto::ProfileName>,
    sample: &UsageSampleKind,
) {
    if let Some(agent) = report
        .agents
        .iter_mut()
        .find(|agent| agent.harness == harness && agent.profile == profile)
    {
        add(&mut agent.totals, sample);
    } else if report.agents.len() < 256 {
        let mut totals = UsageTotals::default();
        add(&mut totals, sample);
        report.agents.push(vessel_proto::usage::AgentUsage {
            harness,
            profile,
            totals,
        });
    } else {
        report.incomplete = true;
    }
}

/// A whole increment must not be assigned to a period that cuts its
/// observation interval. The original sample still explains the gap.
pub(super) fn add_selected(
    report: &mut vessel_proto::usage::UsageReport,
    sample: &vessel_proto::usage::UsageSample,
    harness: vessel_proto::HarnessRef,
    profile: Option<vessel_proto::ProfileName>,
    from: chrono::DateTime<chrono::Utc>,
) {
    let partial = match &sample.kind {
        UsageSampleKind::Cost { since, cost } if *since < from => {
            Some(UsageSampleKind::CostBaseline { cost: cost.clone() })
        }
        UsageSampleKind::Cost { .. }
        | UsageSampleKind::Turn { .. }
        | UsageSampleKind::CostBaseline { .. }
        | UsageSampleKind::CostUnavailable => None,
    };
    let attributed = partial.as_ref().unwrap_or(&sample.kind);
    add(&mut report.totals, attributed);
    add_agent(report, harness, profile, attributed);
}
