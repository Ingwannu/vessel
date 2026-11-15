//! Factual saved-login notices. A full subscription bar is not proof a turn
//! stopped: credits or extra usage may still permit it, and an ACP process
//! may remember an older login. No recovery action is advertised here.
use chrono::{DateTime, Utc};
use vessel_proto::limits::{ExtraUsage, LimitNotice, LimitScope, LimitSnapshot};

pub(super) fn notice(snapshot: &LimitSnapshot, now: DateTime<Utc>) -> Option<LimitNotice> {
    if snapshot.allowed == Some(true)
        || snapshot.extra_usage == Some(ExtraUsage::Enabled)
        || snapshot.credits.as_ref().is_some_and(|credits| {
            credits.has_credits == Some(true) || credits.unlimited == Some(true)
        })
    {
        return None;
    }
    let full: Vec<_> = snapshot
        .windows
        .iter()
        .filter(|window| {
            matches!(window.scope, LimitScope::General)
                && window.used.is_some_and(|used| used.get() >= 100.0)
        })
        .collect();
    if full.is_empty() {
        return None;
    }
    let latest_reported_reset = full
        .iter()
        .map(|window| window.resets_at.filter(|at| *at > now))
        .collect::<Option<Vec<_>>>()
        .and_then(|resets| resets.into_iter().max());
    Some(LimitNotice {
        reported_full: full.iter().map(|window| window.id.clone()).collect(),
        latest_reported_reset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::limits::{ExtraUsage, LimitCredits};
    fn snapshot() -> LimitSnapshot {
        serde_json::from_str(r#"{"at":"2026-06-01T12:00:00Z","plan":null,"windows":[{"id":"five_hour","period":{"kind":"five_hour"},"slot":"only","scope":{"kind":"general"},"used":100,"resets_at":"2026-06-01T14:00:00Z"},{"id":"weekly","period":{"kind":"weekly"},"slot":"only","scope":{"kind":"general"},"used":125,"resets_at":"2026-06-04T00:00:00Z"}],"credits":null,"extra_usage":null,"allowed":null,"limit_reached":null,"incomplete":false}"#).unwrap()
    }
    fn now() -> DateTime<Utc> {
        "2026-06-01T12:00:00Z".parse().unwrap()
    }
    #[test]
    fn a_notice_names_all_full_windows_and_latest_known_future_reset() {
        assert_eq!(
            notice(&snapshot(), now()),
            Some(LimitNotice {
                reported_full: vec!["five_hour".parse().unwrap(), "weekly".parse().unwrap()],
                latest_reported_reset: Some("2026-06-04T00:00:00Z".parse().unwrap())
            })
        );
        let mut data = snapshot();
        data.windows.first_mut().unwrap().resets_at = None;
        assert_eq!(notice(&data, now()).unwrap().latest_reported_reset, None);
        data.windows.first_mut().unwrap().resets_at = Some(now());
        assert_eq!(notice(&data, now()).unwrap().latest_reported_reset, None);
    }
    #[test]
    fn explicit_paid_or_provider_permission_prevents_a_false_limit_notice() {
        let mut data = snapshot();
        data.allowed = Some(true);
        assert_eq!(notice(&data, now()), None);
        data.allowed = None;
        data.extra_usage = Some(ExtraUsage::Enabled);
        assert_eq!(notice(&data, now()), None);
        data.extra_usage = None;
        data.credits = Some(LimitCredits {
            balance: None,
            has_credits: Some(true),
            unlimited: None,
        });
        assert_eq!(notice(&data, now()), None);
        data.credits.as_mut().unwrap().has_credits = None;
        data.credits.as_mut().unwrap().unlimited = Some(true);
        assert_eq!(notice(&data, now()), None);
    }
    #[test]
    fn scoped_and_code_review_windows_do_not_block_general_work() {
        let mut data = snapshot();
        for window in &mut data.windows {
            window.scope = LimitScope::CodeReview;
        }
        assert_eq!(notice(&data, now()), None);
        for window in &mut data.windows {
            window.scope = LimitScope::Model {
                id: Some("specific".parse().unwrap()),
                name: "Specific model".parse().unwrap(),
            };
        }
        assert_eq!(notice(&data, now()), None);
    }
}
