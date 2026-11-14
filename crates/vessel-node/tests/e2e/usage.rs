//! Usage through a real node and ACP fake: the same reports as the
//! vendor protocol, without a credential read or a vendor request.
use crate::agents::{as_agent, credential, structured, vessel_mcp};
use crate::secure::{device_via, dialer, direct_route};
use crate::support::{Fixture, ask, client, within};
use vessel_client::Connector as _;
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageScope};
use vessel_proto::{ErrorCode, EventSeq, Scope, SessionId};
use vessel_rpc::CallError;

fn session(id: SessionId) -> QueryUsage {
    QueryUsage {
        scope: UsageScope::Session { session: id },
        period: UsagePeriod::Session,
        after: None,
    }
}
fn code<T: std::fmt::Debug>(result: Result<T, CallError>) -> ErrorCode {
    match result {
        Err(CallError::Remote(error)) => error.code,
        other => panic!("expected refusal, got {other:?}"),
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reports_survive_restart_and_viewers_never_get_cost() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let running = structured(&owner, &fixture).await;
        // `credential` is not needed here. Ordinary fake usage has a
        // cumulative cost of .5, so sending it twice must not total 1.0.
        let id = running.id;
        let mut stream = owner.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        assert_eq!(
            ask(&owner, &mut stream, &mut seen, id, "usage").await,
            "used"
        );
        assert_eq!(
            ask(&owner, &mut stream, &mut seen, id, "usage").await,
            "used"
        );
        let usage = owner.usage(session(id)).await.unwrap();
        assert_eq!(usage.totals.reported.get(), 2);
        assert_eq!(usage.totals.tokens.unwrap().total.get(), 60);
        assert_eq!(
            usage.totals.costs[0].amount,
            vessel_proto::conversation::FiniteNumber::new(0.5).unwrap()
        );
        let viewer = read_only(&owner, &node).await;
        let visible = viewer.usage(session(id)).await.unwrap();
        assert_eq!(visible.totals.tokens, usage.totals.tokens);
        assert!(visible.totals.costs.is_empty());
        assert!(
            visible
                .agents
                .iter()
                .all(|agent| agent.totals.costs.is_empty())
        );
        assert!(visible.current_costs.is_empty());
        assert_eq!(
            ask(&owner, &mut stream, &mut seen, id, "usage-burst").await,
            "used"
        );
        let sparse = viewer
            .usage(QueryUsage {
                after: usage.samples.last().map(|sample| sample.seq),
                ..session(id)
            })
            .await
            .unwrap();
        assert!(sparse.samples.is_empty());
        assert!(sparse.next.is_some(), "cost redaction is not EOF");
        let second = viewer
            .usage(QueryUsage {
                after: sparse.next,
                ..session(id)
            })
            .await
            .unwrap();
        assert!(second.next.is_none());
        assert_eq!(second.totals, sparse.totals);
        let usage = owner.usage(session(id)).await.unwrap();
        assert_eq!(
            code(owner.usage(session(SessionId::random())).await),
            ErrorCode::NotFound
        );
        assert_eq!(
            code(
                owner
                    .usage(QueryUsage {
                        scope: UsageScope::Machine,
                        period: UsagePeriod::Session,
                        after: None
                    })
                    .await
            ),
            ErrorCode::InvalidRequest
        );
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        assert_eq!(owner.usage(session(id)).await.unwrap().totals, usage.totals);
        let machine = owner
            .usage(QueryUsage {
                scope: UsageScope::Machine,
                period: UsagePeriod::Day,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(machine.totals, usage.totals);
        node.stop().await.unwrap();
    })
    .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_cannot_read_machine_or_another_sessions_usage() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut own = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut own).await;
        let agent = as_agent(&socket, &auth).await;
        assert!(agent.usage(session(own.id)).await.is_ok());
        assert_eq!(
            code(
                agent
                    .usage(QueryUsage {
                        scope: UsageScope::Machine,
                        period: UsagePeriod::Day,
                        after: None
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        let other = structured(&owner, &fixture).await;
        assert_eq!(
            code(agent.usage(session(other.id)).await),
            ErrorCode::Forbidden
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_load_cost_before_ready_is_only_observed_once() {
    within(async {
        let fixture = Fixture::new();
        std::fs::create_dir_all(fixture.data_path()).unwrap();
        std::fs::write(
            fixture.data_dir().profiles_path(),
            "[profiles.replay]\nharness = 'claude'\nenv = { FAKE_ACP_LOAD_USAGE = '1' }\n",
        )
        .unwrap();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let mut spec = crate::support::structured_spec(&fixture, None);
        spec.profile = Some("replay".parse().unwrap());
        let id = owner.create_session(spec).await.unwrap().id;
        let mut stream = owner.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        assert_eq!(
            ask(&owner, &mut stream, &mut seen, id, "usage").await,
            "used"
        );
        let before = owner.usage(session(id)).await.unwrap();
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let resumed = owner.resume_session(id).await.unwrap();
        let mut stream = owner.follow(id, resumed.next_seq).await.unwrap();
        let mut seen = Vec::new();
        assert_eq!(
            ask(&owner, &mut stream, &mut seen, id, "usage").await,
            "used"
        );
        let after = owner.usage(session(id)).await.unwrap();
        assert_eq!(after.totals.costs, before.totals.costs);
        assert_eq!(after.current_costs, before.current_costs);
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
    // A stock fixture advertises no public route. Pair only over its actual
    // ephemeral loopback listener, as the secure transport tests do.
    offer.direct = vec![direct_route(node)];
    let (paired, _) = pair_over(&dialer(), &offer, "usage viewer", RouteTimeouts::default())
        .await
        .unwrap();
    let route = direct_route(node);
    device_via(dialer(), &paired, &[&route], &[])
        .connect(client())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agents_day_includes_all_profiles_and_a_named_profile_can_be_selected() {
    within(async {
        let fixture = Fixture::new();
        std::fs::create_dir_all(fixture.data_path()).unwrap();
        std::fs::write(
            fixture.data_dir().profiles_path(),
            "[profiles.alt]\nharness = 'claude'\n",
        )
        .unwrap();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        for profile in [None, Some("alt"), Some("claude")] {
            let mut spec = crate::support::structured_spec(&fixture, None);
            spec.profile = profile.map(|name| name.parse().unwrap());
            let id = owner.create_session(spec).await.unwrap().id;
            let mut stream = owner.follow(id, EventSeq::ZERO).await.unwrap();
            assert_eq!(
                ask(&owner, &mut stream, &mut Vec::new(), id, "usage").await,
                "used"
            );
            let _durable = owner.usage(session(id)).await.unwrap();
        }
        let harness = vessel_proto::HarnessKind::ClaudeCode.into();
        let all = owner
            .usage(QueryUsage {
                scope: UsageScope::Agent {
                    harness,
                    profile: None,
                },
                period: UsagePeriod::Day,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(all.totals.reported.get(), 3);
        assert_eq!(all.totals.tokens.unwrap().total.get(), 90);
        let named = owner
            .usage(QueryUsage {
                scope: UsageScope::Agent {
                    harness: vessel_proto::HarnessKind::ClaudeCode.into(),
                    profile: Some("alt".parse().unwrap()),
                },
                period: UsagePeriod::Week,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(named.totals.reported.get(), 1);
        assert_eq!(named.totals.tokens.unwrap().total.get(), 30);
        assert_eq!(named.agents.len(), 1);
        assert_eq!(named.agents[0].profile.as_ref().unwrap().as_str(), "alt");
        node.stop().await.unwrap();
    })
    .await;
}
