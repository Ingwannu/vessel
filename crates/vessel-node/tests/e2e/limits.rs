//! Real node/secure-device calls with synthetic vendor response shapes.
use crate::secure::{device_via, dialer, direct_route};
use crate::support::{Fixture, client, within};
use vessel_client::Connector as _;
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_harness::LimitProbe;
use vessel_proto::{ErrorCode, HarnessKind, Scope, limits::*, usage::CostVisibility};
use vessel_rpc::CallError;
fn query(refresh: LimitRefresh) -> GetUsageLimits {
    GetUsageLimits {
        scope: LimitsScope::Account {
            account: LimitAccount {
                harness: HarnessKind::Codex.into(),
                profile: None,
            },
        },
        refresh,
    }
}
fn setup(fixture: &Fixture) -> LimitProbe {
    let home = fixture.user_home().join(".codex");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("auth.json"),br#"{"tokens":{"access_token":"synthetic-node-limits-offline-login","account_id":"synthetic-node-account"}}"#).unwrap();
    let body = fixture.user_home().join("codex-limits-response.json");
    std::fs::write(&body, vessel_testkit::limits::CODEX_USAGE).unwrap();
    LimitProbe::Recorded {
        root: fixture.user_home().to_owned(),
        body,
        status: 200,
        retry_after: None,
    }
}
fn remote<T: std::fmt::Debug>(result: Result<T, CallError>) -> ErrorCode {
    match result {
        Err(CallError::Remote(error)) => error.code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn limits_are_opt_in_private_readable_and_forgettable_on_a_real_node() {
    within(async {
        let fixture = Fixture::new();
        let probe = setup(&fixture);
        let node = fixture
            .start_with(|config| config.limits = probe.clone())
            .await;
        let owner = fixture.connect(&node).await;
        assert_eq!(
            owner
                .usage_limits(query(LimitRefresh::Refresh))
                .await
                .unwrap()
                .state,
            AccountLimitState::Off
        );
        assert_eq!(
            owner
                .usage_lookup(SetUsageLookup {
                    policy: ExternalLookup::On
                })
                .await
                .unwrap(),
            ExternalLookup::On
        );
        assert_eq!(
            owner
                .usage_limits(query(LimitRefresh::Cached))
                .await
                .unwrap()
                .state,
            AccountLimitState::NotRequested
        );
        let report = owner
            .usage_limits(query(LimitRefresh::Refresh))
            .await
            .unwrap();
        let AccountLimitState::Ready { snapshot } = &report.state else {
            panic!("expected recorded quota response")
        };
        assert_eq!(snapshot.windows.len(), 3);
        assert_eq!(
            snapshot.credits.as_ref().unwrap().balance,
            Some(vessel_proto::conversation::FiniteNumber::new(0.0).unwrap())
        );
        assert_read_only(&owner, &node, &report).await;
        // A changed response retains the same observed login, not a clean 0%.
        std::fs::write(
            fixture.user_home().join("codex-limits-response.json"),
            b"<html>changed response</html>",
        )
        .unwrap();
        let failed = owner
            .usage_limits(query(LimitRefresh::Refresh))
            .await
            .unwrap();
        assert_eq!(
            failed.state,
            AccountLimitState::Failed {
                reason: LimitFailure::ChangedResponse,
                retained: Some(snapshot.clone()),
                retry_at: None
            }
        );
        std::fs::remove_file(fixture.user_home().join(".codex").join("auth.json")).unwrap();
        assert_eq!(
            owner
                .usage_limits(query(LimitRefresh::Refresh))
                .await
                .unwrap()
                .state,
            AccountLimitState::Failed {
                reason: LimitFailure::NoLogin,
                retained: None,
                retry_at: None
            }
        );
        assert_eq!(
            owner
                .usage_lookup(SetUsageLookup {
                    policy: ExternalLookup::Off
                })
                .await
                .unwrap(),
            ExternalLookup::Off
        );
        node.stop().await.unwrap();
        let node = fixture.start_with(|config| config.limits = probe).await;
        let owner = fixture.connect(&node).await;
        assert_eq!(
            owner
                .usage_limits(query(LimitRefresh::Cached))
                .await
                .unwrap()
                .state,
            AccountLimitState::Off
        );
        node.stop().await.unwrap();
    })
    .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn vendor_cooldown_applies_to_forced_refresh_and_skips_new_login_reads() {
    within(async {
        let fixture = Fixture::new();
        let mut probe = setup(&fixture);
        if let LimitProbe::Recorded {
            status,
            retry_after,
            ..
        } = &mut probe
        {
            *status = 429;
            *retry_after = Some("600".to_owned());
        }
        let node = fixture.start_with(|config| config.limits = probe).await;
        let owner = fixture.connect(&node).await;
        let _saved = owner
            .usage_lookup(SetUsageLookup {
                policy: ExternalLookup::On,
            })
            .await
            .unwrap();
        let first = owner
            .usage_limits(query(LimitRefresh::Refresh))
            .await
            .unwrap();
        assert!(matches!(
            first.state,
            AccountLimitState::Failed {
                reason: LimitFailure::RateLimited,
                retry_at: Some(_),
                retained: None
            }
        ));
        std::fs::remove_file(fixture.user_home().join(".codex").join("auth.json")).unwrap();
        assert_eq!(
            owner
                .usage_limits(query(LimitRefresh::Refresh))
                .await
                .unwrap(),
            first
        );
        node.stop().await.unwrap();
    })
    .await;
}
async fn read_only(
    owner: &vessel_client::NodeClient,
    node: &vessel_node::Node,
) -> vessel_client::NodeClient {
    let mut offer = owner
        .invite_with_scopes(None, 600, vec![Scope::Read])
        .await
        .unwrap()
        .link
        .decode()
        .unwrap();
    offer.direct = vec![direct_route(node)];
    let (paired, _) = pair_over(&dialer(), &offer, "limits viewer", RouteTimeouts::default())
        .await
        .unwrap();
    let route = direct_route(node);
    device_via(dialer(), &paired, &[&route], &[])
        .connect(client())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_sees_only_its_own_local_session_limits_and_never_policy_controls() {
    use crate::agents::{as_agent, credential, structured, vessel_mcp};
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut own = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut own).await;
        let agent = as_agent(&socket, &auth).await;
        let request = |session| GetUsageLimits {
            scope: LimitsScope::Session { session },
            refresh: LimitRefresh::Cached,
        };
        assert_eq!(
            agent.usage_limits(request(own.id)).await.unwrap().state,
            AccountLimitState::Off
        );
        let other = structured(&owner, &fixture).await;
        assert_eq!(
            remote(agent.usage_limits(request(other.id)).await),
            ErrorCode::Forbidden
        );
        assert_eq!(
            remote(agent.usage_limits(query(LimitRefresh::Cached)).await),
            ErrorCode::Forbidden
        );
        assert_eq!(
            remote(
                agent
                    .usage_lookup(SetUsageLookup {
                        policy: ExternalLookup::On
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        node.stop().await.unwrap();
    })
    .await;
}

mod profiles;

mod forwarded;

async fn assert_read_only(
    owner: &vessel_client::NodeClient,
    node: &vessel_node::Node,
    report: &UsageLimits,
) {
    let viewer = read_only(owner, node).await;
    let mut expected = report.clone();
    expected.cost_visibility = CostVisibility::Withheld;
    if let AccountLimitState::Ready { snapshot } = &mut expected.state {
        snapshot.credits.as_mut().unwrap().balance = None;
    }
    assert_eq!(
        viewer
            .usage_limits(query(LimitRefresh::Cached))
            .await
            .unwrap(),
        expected
    );
    assert_eq!(
        remote(viewer.usage_limits(query(LimitRefresh::Refresh)).await),
        ErrorCode::Forbidden
    );
    assert_eq!(
        remote(
            viewer
                .usage_lookup(SetUsageLookup {
                    policy: ExternalLookup::Off
                })
                .await
        ),
        ErrorCode::Forbidden
    );
}

mod provider;
