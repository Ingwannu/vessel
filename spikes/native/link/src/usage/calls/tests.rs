use super::*;
mod peer;
use std::sync::Mutex;
use vessel_proto::{
    Call, Outcome, ProfileInfo, Reply,
    limits::{AccountLimitState, LimitAccount, UsageLimits},
    usage::CostVisibility,
};
fn profile(name: &str) -> ProfileInfo {
    ProfileInfo {
        name: name.parse().unwrap(),
        label: Some("shared label".into()),
        harness: "codex".parse().unwrap(),
        sets: Vec::new(),
        model: None,
    }
}
fn limits(account: LimitAccount) -> UsageLimits {
    UsageLimits {
        policy: vessel_proto::limits::ExternalLookup::On,
        account,
        source: None,
        state: AccountLimitState::NotRequested,
        cost_visibility: CostVisibility::Permitted,
        notice: None,
    }
}
#[tokio::test]
async fn inventory_refreshes_only_the_explicit_profile_and_preserves_other_failures() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&calls);
    let peer = peer::connect(move |call| {
        seen.lock().unwrap().push(call.clone());
        if matches!(&call, Call::ListProfiles(_)) {
            return Outcome::Ok(Reply::ListProfiles(vec![
                profile("first"),
                profile("second"),
            ]));
        }
        let Call::GetUsageLimits(ask) = call else {
            panic!("unexpected fixture method");
        };
        let LimitsScope::Account { account } = ask.scope else {
            panic!("account only");
        };
        if account.profile.as_ref().unwrap().as_str() == "first" {
            Outcome::Err(vessel_proto::WireError::new(
                ErrorCode::Forbidden,
                "private fixture body",
            ))
        } else {
            Outcome::Ok(Reply::GetUsageLimits(limits(account)))
        }
    })
    .await;
    let binding = LimitAccount {
        harness: "codex".parse().unwrap(),
        profile: Some("second".parse().unwrap()),
    };
    let report = inventory(&peer.client, &InventoryRefresh::Account(binding))
        .await
        .unwrap();
    assert_eq!(report.accounts().len(), 2);
    assert!(matches!(
        report.accounts()[0].result,
        Err(RequestFailure::Refused(ErrorCode::Forbidden))
    ));
    assert!(report.accounts()[1].result.is_ok());
    let calls = calls.lock().unwrap();
    let modes: Vec<_> = calls
        .iter()
        .filter_map(|call| {
            if let Call::GetUsageLimits(ask) = call {
                Some(ask.refresh)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(modes, [LimitRefresh::Cached, LimitRefresh::Refresh]);
}
#[tokio::test]
async fn cached_inventory_rejects_a_reply_for_another_account() {
    let peer = peer::connect(|call| {
        if matches!(&call, Call::ListProfiles(_)) {
            return Outcome::Ok(Reply::ListProfiles(vec![profile("first")]));
        }
        let Call::GetUsageLimits(ask) = call else {
            panic!("unexpected fixture method");
        };
        assert_eq!(ask.refresh, LimitRefresh::Cached);
        Outcome::Ok(Reply::GetUsageLimits(limits(LimitAccount {
            harness: "codex".parse().unwrap(),
            profile: Some("other".parse().unwrap()),
        })))
    })
    .await;
    let report = inventory(&peer.client, &InventoryRefresh::Cached)
        .await
        .unwrap();
    assert!(matches!(
        report.accounts()[0].result,
        Err(RequestFailure::InvalidReply)
    ));
}
#[test]
fn raw_node_error_text_never_crosses_the_usage_event_boundary() {
    let raw = "untrusted provider body must stay out of UI and logs";
    let safe = call_problem(CallError::Remote(vessel_proto::WireError::new(
        ErrorCode::Forbidden,
        raw,
    )));
    assert_eq!(safe, RequestFailure::Refused(ErrorCode::Forbidden));
    assert!(!format!("{safe:?}").contains(raw));
}
#[tokio::test]
async fn refresh_all_targets_the_bounded_inventory_and_cached_reads_never_refresh() {
    let modes = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&modes);
    let peer = peer::connect(move |call| {
        if matches!(&call, Call::ListProfiles(_)) {
            return Outcome::Ok(Reply::ListProfiles(vec![
                profile("first"),
                profile("second"),
            ]));
        }
        let Call::GetUsageLimits(ask) = call else {
            panic!("unexpected fixture method")
        };
        seen.lock().unwrap().push(ask.refresh);
        let LimitsScope::Account { account } = ask.scope else {
            panic!("account only")
        };
        Outcome::Ok(Reply::GetUsageLimits(limits(account)))
    })
    .await;
    let all = inventory(&peer.client, &InventoryRefresh::All)
        .await
        .unwrap();
    assert_eq!(all.accounts().len(), 2);
    let cached = inventory(&peer.client, &InventoryRefresh::Cached)
        .await
        .unwrap();
    assert_eq!(cached.accounts().len(), 2);
    assert_eq!(
        *modes.lock().unwrap(),
        [
            LimitRefresh::Refresh,
            LimitRefresh::Refresh,
            LimitRefresh::Cached,
            LimitRefresh::Cached
        ]
    );
}

#[tokio::test]
async fn retired_usage_selection_never_polls_or_retargets_a_mutation() {
    crate::testing::within(async {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&calls);
        let mut peer = peer::connect(move |call| {
            seen.lock().unwrap().push(call);
            Outcome::Ok(Reply::SetUsageLookup(
                vessel_proto::limits::ExternalLookup::Off,
            ))
        })
        .await;
        let pending = peer.client.call(SetUsageLookup {
            policy: vessel_proto::limits::ExternalLookup::Off,
        });
        let connected = peer.state.clients.remove("fixture").unwrap();
        connected.retire();
        // Even reconnecting to the same authenticated node produces a distinct context.
        let new_context = peer
            .state
            .admissions
            .admit(
                connected.context.machine().clone(),
                connected.context.node(),
            )
            .unwrap();
        assert_ne!(connected.context, new_context);
        assert!(matches!(
            pending.await,
            Err(ConnectionFailure::Refused(Refusal::Changed))
        ));
        assert!(calls.lock().unwrap().is_empty());
    })
    .await;
}
