use super::*;
use vessel_proto::{
    limits::{ExternalLookup, LimitAccount},
    usage::CostVisibility,
};
fn snapshot() -> LimitSnapshot {
    LimitSnapshot {
        at: DateTime::UNIX_EPOCH,
        plan: None,
        windows: Vec::new(),
        credits: None,
        extra_usage: None,
        allowed: None,
        limit_reached: None,
        incomplete: false,
    }
}
fn limits(state: AccountLimitState) -> UsageLimits {
    UsageLimits {
        policy: ExternalLookup::On,
        account: LimitAccount {
            harness: "codex".parse().unwrap(),
            profile: Some("codex".parse().unwrap()),
        },
        source: None,
        state,
        cost_visibility: CostVisibility::Permitted,
        notice: None,
    }
}
#[test]
fn failed_and_pending_attempts_cannot_refresh_a_successful_timestamp() {
    let old = snapshot();
    let pending = limits(AccountLimitState::Refreshing {
        retained: Some(old.clone()),
    });
    let view = limit_view(&pending);
    assert_eq!(view.status, LimitStatus::Refreshing);
    assert_eq!(view.snapshot.unwrap().at, old.at);
    let retry = old
        .at
        .checked_add_signed(chrono::Duration::hours(1))
        .unwrap();
    let failed = limits(AccountLimitState::Failed {
        reason: LimitFailure::Rejected,
        retained: Some(old.clone()),
        retry_at: Some(retry),
    });
    let view = limit_view(&failed);
    assert_eq!(view.status, LimitStatus::Failed(LimitFailure::Rejected));
    assert_eq!(view.snapshot.unwrap().at, old.at);
    assert_eq!(view.retry_at, Some(retry));
    let ready = limits(AccountLimitState::Ready {
        snapshot: old.clone(),
    });
    assert_eq!(limit_view(&ready).status, LimitStatus::Current);
}
#[test]
fn off_and_never_checked_are_distinct_and_have_no_snapshot() {
    let off = limits(AccountLimitState::Off);
    let untouched = limits(AccountLimitState::NotRequested);
    assert_eq!(limit_view(&off).status, LimitStatus::Off);
    assert_eq!(limit_view(&untouched).status, LimitStatus::NotRequested);
    assert!(limit_view(&off).snapshot.is_none());
    assert!(limit_view(&untouched).snapshot.is_none());
}
