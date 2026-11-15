//! Whole CLI sentences from typed saved-login facts; no credentials, raw
//! durations or ISO reset strings. UI consumers supply their own language.
use chrono::{DateTime, Utc};
use std::fmt::Write as _;
use vessel_proto::limits::{
    AccountLimitState, ExtraUsage, LimitFailure, LimitPeriod, LimitScope, LimitSnapshot,
    UsageLimits,
};

pub(super) fn render(report: &UsageLimits, now: DateTime<Utc>) -> Result<String, std::fmt::Error> {
    let mut text = String::new();
    writeln!(
        text,
        "{} / {} · saved sign-in limits",
        report.account.harness,
        report
            .account
            .profile
            .as_ref()
            .map_or("default", |profile| profile.as_str())
    )?;
    match report.state {
        AccountLimitState::Off => text.push_str("External limit lookup is off. The usage commands still show what the agent reported.\nAn administrator can run `vessel usage lookup on` to allow this machine to read configured agents' saved access tokens (and Codex account identifiers, when present) and send them only to Anthropic or ChatGPT to read account limits.\n"),
        AccountLimitState::NotRequested => text.push_str("No saved sign-in limits have been checked. Request a refresh to check them.\n"),
        AccountLimitState::Refreshing { ref retained } => { text.push_str("Checking saved sign-in limits.\n"); if let Some(snapshot) = retained { text.push_str("Previous observation, while the check runs:\n"); snapshot_lines(&mut text,snapshot,now)?; } }
        AccountLimitState::Ready { ref snapshot } => snapshot_lines(&mut text,snapshot,now)?,
        AccountLimitState::Failed { reason, ref retained, retry_at } => {
            writeln!(text,"{}",failure(reason))?;
            if let Some(at) = retry_at {
                if at <= now { text.push_str("Check again now.\n"); }
                else { let seconds=u64::try_from(at.signed_duration_since(now).num_seconds()).unwrap_or_default();writeln!(text,"Check again in {}.",duration_words(seconds).to_lowercase())?; }
            }
            if let Some(snapshot) = retained { text.push_str("Previous observation; the latest check failed:\n"); snapshot_lines(&mut text,snapshot,now)?; }
        }
    }
    if report.cost_visibility == vessel_proto::usage::CostVisibility::Withheld {
        text.push_str("Credit balances are hidden for this device.\n");
    }
    Ok(text)
}
fn snapshot_lines(
    text: &mut String,
    snapshot: &LimitSnapshot,
    now: DateTime<Utc>,
) -> Result<(), std::fmt::Error> {
    if snapshot.at > now {
        text.push_str("The observation time is ahead of this device's clock.\n");
    } else {
        let age =
            u64::try_from(now.signed_duration_since(snapshot.at).num_seconds()).unwrap_or_default();
        writeln!(text, "Checked {} ago.", duration_words(age).to_lowercase())?;
    }
    if let Some(plan) = &snapshot.plan {
        writeln!(text, "Plan: {plan}")?;
    }
    if snapshot.windows.is_empty() {
        text.push_str("The provider reported no usage windows.\n");
    }
    for window in &snapshot.windows {
        let period = match window.period {
            LimitPeriod::FiveHour => "5-hour".to_owned(),
            LimitPeriod::Weekly => "Weekly".to_owned(),
            LimitPeriod::Other { duration } => format!(
                "{} period",
                duration_words(duration.as_duration().as_secs())
            ),
            LimitPeriod::Unknown => match window.slot {
                vessel_proto::limits::LimitSlot::Primary => "Primary limit, period not reported",
                vessel_proto::limits::LimitSlot::Secondary => {
                    "Secondary limit, period not reported"
                }
                vessel_proto::limits::LimitSlot::Only => "Period not reported",
            }
            .to_owned(),
        };
        let scope = match &window.scope {
            LimitScope::General => "Account",
            LimitScope::Model { name, .. }
            | LimitScope::Surface { name, .. }
            | LimitScope::Feature { name, .. } => name.as_str(),
            LimitScope::CodeReview => "Code review",
        };
        write!(text, "{scope} · {period}: ")?;
        match window.used {
            Some(used) => write!(text, "{}% used", used.get())?,
            None => text.push_str("usage not reported"),
        }
        match window.resets_at {
            Some(at) => writeln!(text, " · {}", reset(now, at))?,
            None => text.push_str(" · reset not reported\n"),
        }
    }
    if let Some(credits) = &snapshot.credits {
        if let Some(balance) = credits.balance {
            writeln!(text, "{} credits reported", balance.get())?;
        }
        if credits.has_credits == Some(true) {
            text.push_str("The provider reports credits available for continued use.\n");
        }
        if credits.unlimited == Some(true) {
            text.push_str("The provider reports unlimited credit use.\n");
        }
    }
    match snapshot.extra_usage {
        Some(ExtraUsage::Enabled) => text.push_str("Additional paid usage is enabled.\n"),
        Some(ExtraUsage::Disabled) => text.push_str("Additional paid usage is disabled.\n"),
        None => {}
    }
    match snapshot.allowed {
        Some(true) => text.push_str("The provider reports continued use is allowed.\n"),
        Some(false) => text.push_str("The provider reports continued use is not allowed.\n"),
        None => {}
    }
    if snapshot.limit_reached == Some(true) {
        text.push_str("The provider reports a subscription limit reached; check paid-use eligibility above.\n");
    }
    if snapshot.incomplete {
        text.push_str("Some reported limits could not be read; this list is incomplete.\n");
    }
    text.push_str("These are limits of the observed saved sign-in; a running agent may remember another login.\n");
    Ok(())
}
fn reset(now: DateTime<Utc>, at: DateTime<Utc>) -> String {
    if at <= now {
        return "reset time has passed; refresh to check".to_owned();
    }
    let seconds = u64::try_from(at.signed_duration_since(now).num_seconds()).unwrap_or_default();
    if seconds < 60 {
        return "resets in less than 1 min".to_owned();
    }
    let minutes = seconds.div_ceil(60);
    let hours = minutes.checked_div(60).unwrap_or_default();
    let remainder = minutes.checked_rem(60).unwrap_or_default();
    if hours == 0 {
        format!("resets in {minutes} min")
    } else if remainder == 0 {
        format!("resets in {hours} h")
    } else {
        format!("resets in {hours} h {remainder} min")
    }
}
fn failure(reason: LimitFailure) -> &'static str {
    match reason {
        LimitFailure::NoLogin => {
            "No saved OAuth sign-in was found. Sign in with the agent, then refresh."
        }
        LimitFailure::Expired => {
            "The saved sign-in has expired. Open the agent to refresh its sign-in, then try again."
        }
        LimitFailure::Rejected => {
            "The provider rejected the saved sign-in. Sign in with the agent again, then refresh."
        }
        LimitFailure::Unreachable => {
            "The provider could not be reached. Check this machine's connection, then refresh."
        }
        LimitFailure::ChangedResponse => {
            "The provider's limits could not be read. Try again later or update Vessel."
        }
        LimitFailure::Unsupported => {
            "This agent's saved sign-in limits are not supported here. Check the agent's own usage page."
        }
        LimitFailure::UnsafeLoginStore => {
            "The configured saved sign-in could not be read safely. Check its location and permissions on this machine."
        }
        LimitFailure::DifferentRoute => {
            "This profile uses another provider route. Check that provider's own usage page."
        }
        LimitFailure::Unassociated => {
            "The saved sign-in for this session cannot be identified. Choose an account in Usage to check its limits."
        }
        LimitFailure::Busy => {
            "Limit checks are busy. Wait for the current checks to finish, then try again."
        }
        LimitFailure::RateLimited => {
            "The provider is limiting status checks. Wait until the next permitted check."
        }
    }
}

fn duration_words(seconds: u64) -> String {
    if seconds < 60 {
        return "Less than 1 min".to_owned();
    }
    let minutes = seconds.div_ceil(60);
    let hours = minutes.checked_div(60).unwrap_or_default();
    let remainder = minutes.checked_rem(60).unwrap_or_default();
    if hours == 0 {
        format!("{minutes} min")
    } else if remainder == 0 {
        format!("{hours} h")
    } else {
        format!("{hours} h {remainder} min")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{HarnessKind, limits::*, usage::CostVisibility};
    fn now() -> DateTime<Utc> {
        "2026-06-01T12:00:00Z".parse().unwrap()
    }
    fn report(state: AccountLimitState) -> UsageLimits {
        UsageLimits {
            policy: ExternalLookup::On,
            account: LimitAccount {
                harness: HarnessKind::Codex.into(),
                profile: None,
            },
            source: Some(LimitLoginStore::Codex),
            state,
            cost_visibility: CostVisibility::Permitted,
            notice: None,
        }
    }
    #[test]
    fn full_paid_subscription_output_does_not_claim_the_session_stopped() {
        let snapshot:LimitSnapshot=serde_json::from_str(r#"{"at":"2026-06-01T12:00:00Z","plan":"Plus","windows":[{"id":"weekly","period":{"kind":"weekly"},"slot":"primary","scope":{"kind":"general"},"used":100,"resets_at":"2026-06-01T14:10:00Z"}],"credits":{"balance":0,"has_credits":true,"unlimited":false},"extra_usage":null,"allowed":true,"limit_reached":true,"incomplete":false}"#).unwrap();
        assert_eq!(
            render(&report(AccountLimitState::Ready { snapshot }), now()).unwrap(),
            "codex / default · saved sign-in limits\nChecked less than 1 min ago.\nPlan: Plus\nAccount · Weekly: 100% used · resets in 2 h 10 min\n0 credits reported\nThe provider reports credits available for continued use.\nThe provider reports continued use is allowed.\nThe provider reports a subscription limit reached; check paid-use eligibility above.\nThese are limits of the observed saved sign-in; a running agent may remember another login.\n"
        );
    }
    #[test]
    fn unknown_period_missing_percent_and_hidden_balance_keep_their_meaning() {
        let snapshot:LimitSnapshot=serde_json::from_str(r#"{"at":"2026-06-01T12:00:00Z","plan":null,"windows":[{"id":"unknown_primary","period":{"kind":"unknown"},"slot":"primary","scope":{"kind":"general"},"used":null,"resets_at":null}],"credits":null,"extra_usage":null,"allowed":null,"limit_reached":null,"incomplete":true}"#).unwrap();
        let mut data = report(AccountLimitState::Ready { snapshot });
        data.cost_visibility = CostVisibility::Withheld;
        assert_eq!(
            render(&data, now()).unwrap(),
            "codex / default · saved sign-in limits\nChecked less than 1 min ago.\nAccount · Primary limit, period not reported: usage not reported · reset not reported\nSome reported limits could not be read; this list is incomplete.\nThese are limits of the observed saved sign-in; a running agent may remember another login.\nCredit balances are hidden for this device.\n"
        );
    }
    #[test]
    fn checking_cooldown_is_distinct_from_a_subscription_reset() {
        let state = AccountLimitState::Failed {
            reason: LimitFailure::RateLimited,
            retained: None,
            retry_at: Some("2026-06-01T12:10:00Z".parse().unwrap()),
        };
        assert_eq!(
            render(&report(state), now()).unwrap(),
            "codex / default · saved sign-in limits\nThe provider is limiting status checks. Wait until the next permitted check.\nCheck again in 10 min.\n"
        );
    }
}
