//! `vessel usage`: the node owns accounting; this only selects and prints.
use crate::args::{LookupChoice, Period};
mod limits;
mod limits_render;
use crate::args::{Target, UsageCommand};
use crate::{connect::connect, error::CliError, io};
use std::fmt::Write as _;
use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageSampleKind, UsageScope, UsageTotals};

pub(crate) async fn run(target: &Target, command: UsageCommand) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let (scope, period, after) = match command {
        UsageCommand::Limits {
            harness,
            profile,
            refresh,
        } => {
            return limits::account(
                &client,
                vessel_proto::limits::LimitAccount { harness, profile },
                refresh,
            )
            .await;
        }
        UsageCommand::SessionLimits { session, refresh } => {
            return limits::session(&client, session.resolve(&client).await?.id, refresh).await;
        }
        UsageCommand::Lookup { policy } => {
            return limits::lookup(
                &client,
                match policy {
                    LookupChoice::Off => vessel_proto::limits::ExternalLookup::Off,
                    LookupChoice::On => vessel_proto::limits::ExternalLookup::On,
                },
            )
            .await;
        }
        UsageCommand::Machine { period } => (UsageScope::Machine, from(period), None),
        UsageCommand::Agent {
            harness,
            profile,
            period,
        } => (UsageScope::Agent { harness, profile }, from(period), None),
        UsageCommand::Session { session, after } => (
            UsageScope::Session {
                session: session.resolve(&client).await?.id,
            },
            UsagePeriod::Session,
            after,
        ),
    };
    let report = client
        .usage(QueryUsage {
            scope,
            period,
            after,
        })
        .await?;
    let mut text = match period {
        UsagePeriod::Day => "Past 24 hours\n".to_owned(),
        UsagePeriod::Week => "Past 7 days\n".to_owned(),
        UsagePeriod::Session => "Retained session usage\n".to_owned(),
    };
    totals(&mut text, &report.totals)?;
    for cost in &report.current_costs {
        writeln!(
            text,
            "{} {} latest cumulative session cost",
            cost.amount.get(),
            cost.unit
        )?;
    }
    if report.cost_visibility == vessel_proto::usage::CostVisibility::Withheld {
        text.push_str(
            "Cost is hidden for this device. Ask the machine owner for permission to view costs.\n",
        );
    }
    writeln!(
        text,
        "Accounting began {}. Data kept for at most 90 days.",
        report.accounting_started.format("%Y-%m-%d %H:%M UTC")
    )?;
    if report.incomplete {
        text.push_str("Coverage may be incomplete for this range.\n");
    }
    for agent in report.agents {
        writeln!(
            text,
            "{} / {}",
            agent.harness,
            agent
                .profile
                .as_ref()
                .map_or("default", |profile| profile.as_str())
        )?;
        totals(&mut text, &agent.totals)?;
    }
    for sample in &report.samples {
        sample_line(&mut text, sample)?;
    }
    if let Some(next) = report.next {
        writeln!(text, "More reports: repeat with --after {next}")?;
    }
    Ok(io::out(text.as_bytes()).await?)
}
fn from(period: Period) -> UsagePeriod {
    match period {
        Period::Day => UsagePeriod::Day,
        Period::Week => UsagePeriod::Week,
    }
}
fn totals(text: &mut String, total: &UsageTotals) -> Result<(), std::fmt::Error> {
    if total.token_accounting == vessel_proto::usage::TokenAccounting::Overflow {
        text.push_str("Token totals exceed the counter range and are partial.\n");
    }
    let turns = total.reported.get().saturating_add(total.unreported.get());
    if let Some(tokens) = total.tokens {
        writeln!(
            text,
            "{} input, {} output, {} total tokens (reported turns: {}; unreported turns: {})",
            tokens.input.get(),
            tokens.output.get(),
            tokens.total.get(),
            total.reported.get(),
            total.unreported.get()
        )?;
        if let Some(thought) = tokens.thought {
            writeln!(text, "{} reasoning tokens", thought.get())?;
            if total.optional_reports.thought.get() < turns {
                writeln!(
                    text,
                    "Reasoning reported for {} of {turns} turns",
                    total.optional_reports.thought.get()
                )?;
            }
        } else {
            text.push_str("Reasoning tokens not reported.\n");
        }
        if let Some(read) = tokens.cached_read {
            writeln!(text, "{} tokens read from cache", read.get())?;
            if total.optional_reports.cached_read.get() < turns {
                writeln!(
                    text,
                    "Cache reads reported for {} of {turns} turns",
                    total.optional_reports.cached_read.get()
                )?;
            }
        }
        if let Some(write) = tokens.cached_write {
            writeln!(text, "{} tokens written to cache", write.get())?;
            if total.optional_reports.cached_write.get() < turns {
                writeln!(
                    text,
                    "Cache writes reported for {} of {turns} turns",
                    total.optional_reports.cached_write.get()
                )?;
            }
        }
    } else {
        writeln!(
            text,
            "No recorded token counts for this period (reported turns: {}; unreported turns: {}).",
            total.reported.get(),
            total.unreported.get()
        )?;
    }
    if total.unaccounted_costs.get() > 0 {
        text.push_str("Some reported costs could not be totaled.\n");
    }
    for cost in &total.costs {
        writeln!(text, "{} {} reported cost", cost.amount.get(), cost.unit)?;
    }
    Ok(())
}

fn sample_line(
    text: &mut String,
    sample: &vessel_proto::usage::UsageSample,
) -> Result<(), std::fmt::Error> {
    write!(text, "Report {}: ", sample.seq)?;
    match &sample.kind {
        UsageSampleKind::Turn { usage: Some(usage) } => {
            writeln!(
                text,
                "{} input, {} output, {} total tokens",
                usage.input.get(),
                usage.output.get(),
                usage.total.get()
            )?;
            sample_optional(
                text,
                usage.thought,
                "reasoning tokens",
                "reasoning tokens not reported",
            )?;
            sample_optional(
                text,
                usage.cached_read,
                "tokens read from cache",
                "cache reads not reported",
            )?;
            sample_optional(
                text,
                usage.cached_write,
                "tokens written to cache",
                "cache writes not reported",
            )?;
        }
        UsageSampleKind::Turn { usage: None } => text.push_str("tokens not reported\n"),
        UsageSampleKind::CostUnavailable => {
            text.push_str("cost cannot be totaled at this precision\n");
        }
        UsageSampleKind::CostBaseline { cost } => {
            writeln!(
                text,
                "{} {} cumulative cost; earlier timing unknown",
                cost.amount.get(),
                cost.unit
            )?;
        }
        UsageSampleKind::Cost { cost, since } => {
            writeln!(
                text,
                "+{} {} since {}",
                cost.amount.get(),
                cost.unit,
                since
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M %Z")
            )?;
        }
    }
    Ok(())
}

fn sample_optional(
    text: &mut String,
    value: Option<vessel_proto::TokenCount>,
    label: &str,
    absent: &str,
) -> Result<(), std::fmt::Error> {
    if let Some(value) = value {
        writeln!(text, "  {} {label}", value.get())
    } else {
        writeln!(text, "  {absent}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::usage::{OptionalUsageReports, ReportCount, UsageSample};
    use vessel_proto::{EventSeq, TokenCount, TurnUsage};

    #[test]
    fn missing_counts_preserve_the_number_of_unreported_turns() {
        let mut text = String::new();
        totals(
            &mut text,
            &UsageTotals {
                unreported: ReportCount::new(3),
                ..UsageTotals::default()
            },
        )
        .unwrap();
        assert_eq!(
            text,
            "No recorded token counts for this period (reported turns: 0; unreported turns: 3).\n"
        );
    }

    #[test]
    fn reasoning_zero_absence_and_partial_coverage_are_distinct() {
        let base = TurnUsage {
            total: TokenCount::new(30),
            input: TokenCount::new(20),
            output: TokenCount::new(10),
            thought: Some(TokenCount::new(0)),
            cached_read: None,
            cached_write: None,
        };
        let mut text = String::new();
        totals(
            &mut text,
            &UsageTotals {
                tokens: Some(base),
                reported: ReportCount::new(2),
                unreported: ReportCount::new(1),
                optional_reports: OptionalUsageReports {
                    thought: ReportCount::new(1),
                    ..OptionalUsageReports::default()
                },
                ..UsageTotals::default()
            },
        )
        .unwrap();
        assert_eq!(
            text,
            "20 input, 10 output, 30 total tokens (reported turns: 2; unreported turns: 1)\n0 reasoning tokens\nReasoning reported for 1 of 3 turns\n"
        );
        let mut absent = base;
        absent.thought = None;
        text.clear();
        totals(
            &mut text,
            &UsageTotals {
                tokens: Some(absent),
                reported: ReportCount::new(1),
                ..UsageTotals::default()
            },
        )
        .unwrap();
        assert_eq!(
            text,
            "20 input, 10 output, 30 total tokens (reported turns: 1; unreported turns: 0)\nReasoning tokens not reported.\n"
        );
        text.clear();
        for (seq, usage) in [(0, Some(base)), (1, Some(absent)), (2, None)] {
            sample_line(
                &mut text,
                &UsageSample {
                    seq: EventSeq::new(seq),
                    at: chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
                    kind: UsageSampleKind::Turn { usage },
                },
            )
            .unwrap();
        }
        assert_eq!(
            text,
            "Report 0: 20 input, 10 output, 30 total tokens\n  0 reasoning tokens\n  cache reads not reported\n  cache writes not reported\nReport 1: 20 input, 10 output, 30 total tokens\n  reasoning tokens not reported\n  cache reads not reported\n  cache writes not reported\nReport 2: tokens not reported\n"
        );
    }
}
