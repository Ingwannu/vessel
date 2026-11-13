//! Aggregation of reported fields. Cache values remain provider-defined;
//! they are never added again to input or total tokens.

use super::money::Amount;
use vessel_proto::conversation::{Cost, CostUnit};
use vessel_proto::usage::{AgentUsage, ReportCount, UsageReport};
use vessel_proto::{HarnessRef, ProfileName};

#[derive(Debug, Default)]
pub(super) struct Total {
    value: UsageTotals,
    costs: Vec<CostTotal>,
}
#[derive(Debug)]
struct CostTotal {
    unit: CostUnit,
    amount: Amount,
    reports: ReportCount,
}
impl Total {
    pub(super) fn add(&mut self, sample: &UsageSampleKind) {
        if let UsageSampleKind::Cost { cost, .. } = sample {
            self.add_cost(cost);
        } else {
            add_tokens(&mut self.value, sample);
        }
    }
    fn add_cost(&mut self, cost: &Cost) {
        let Some(amount) = Amount::reported(cost.amount) else {
            self.value.unaccounted_costs.increment();
            return;
        };
        if let Some(previous) = self
            .costs
            .iter_mut()
            .find(|previous| previous.unit == cost.unit)
        {
            previous.amount.add(amount);
            previous.reports.increment();
        } else if self.costs.len() < 32 {
            self.costs.push(CostTotal {
                unit: cost.unit.clone(),
                amount,
                reports: ReportCount::new(1),
            });
        } else {
            self.value.unaccounted_costs.increment();
        }
    }
    pub(super) fn finish(mut self) -> UsageTotals {
        for cost in self.costs {
            if let Some(amount) = cost.amount.wire() {
                self.value.costs.push(Cost {
                    amount,
                    unit: cost.unit,
                });
            } else {
                self.value.unaccounted_costs = ReportCount::new(
                    self.value
                        .unaccounted_costs
                        .get()
                        .saturating_add(cost.reports.get()),
                );
            }
        }
        self.value
            .costs
            .sort_by(|a, b| a.unit.as_str().cmp(b.unit.as_str()));
        self.value
    }
}
use vessel_proto::usage::{TokenAccounting, UsageSampleKind, UsageTotals};
use vessel_proto::{TokenCount, TurnUsage};

fn add_tokens(total: &mut UsageTotals, sample: &UsageSampleKind) {
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
        UsageSampleKind::Cost { .. } => {}
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

/// Cost accumulators remain internal until the entire selected scan ends.
#[derive(Debug, Default)]
pub(super) struct Aggregation {
    total: Total,
    agents: Vec<AgentTotal>,
    incomplete: bool,
}
#[derive(Debug)]
struct AgentTotal {
    harness: HarnessRef,
    profile: Option<ProfileName>,
    total: Total,
}
impl Aggregation {
    /// A whole increment must not be assigned to a period that cuts its
    /// observation interval. The original sample still explains the gap.
    pub(super) fn add_selected(
        &mut self,
        sample: &vessel_proto::usage::UsageSample,
        harness: HarnessRef,
        profile: Option<ProfileName>,
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
        self.total.add(attributed);
        if let Some(agent) = self
            .agents
            .iter_mut()
            .find(|agent| agent.harness == harness && agent.profile == profile)
        {
            agent.total.add(attributed);
        } else if self.agents.len() < 256 {
            let mut total = Total::default();
            total.add(attributed);
            self.agents.push(AgentTotal {
                harness,
                profile,
                total,
            });
        } else {
            self.incomplete = true;
        }
    }
    pub(super) fn finish(self, report: &mut UsageReport) {
        report.incomplete |= self.incomplete;
        report.totals = self.total.finish();
        report.agents = self
            .agents
            .into_iter()
            .map(|agent| AgentUsage {
                harness: agent.harness,
                profile: agent.profile,
                totals: agent.total.finish(),
            })
            .collect();
    }
}
