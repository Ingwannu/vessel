//! A real admitted agent can steer its child, while scope and ancestry stay independent.
use super::*;
use crate::principal::Principal;
use vessel_proto::launch_profiles::ApplySessionOptions;
use vessel_proto::{ErrorCode, Handler as _, Scope};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_bundle_preserves_agent_child_ownership_and_operate_without_read() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let owner = fixture.context(Principal::Owner).await;
            let parent = fixture.running(&owner.connection).await;
            let agent = fixture
                .context(Principal::Agent {
                    node: fixture.node.as_ref().unwrap().info().id,
                    session: parent.id(),
                    scopes: vec![Scope::Read, Scope::Operate],
                })
                .await;
            let child = fixture.running(&agent.connection).await;
            let service = fixture.service();
            let request = |session| ApplySessionOptions {
                session,
                options: bundle(&[("mode", "plan")]),
            };
            let refused = phase(
                "agent cannot steer its own parent",
                service.apply_session_options(&agent.connection, request(parent.id())),
            )
            .await
            .unwrap_err();
            assert_eq!(refused.code, ErrorCode::Forbidden);
            let applied = phase(
                "agent can steer child it created",
                service.apply_session_options(&agent.connection, request(child.id())),
            )
            .await
            .unwrap();
            assert!(applied.failure().is_none());
            assert_eq!(
                applied.applied().as_slice(),
                bundle(&[("mode", "plan")]).as_slice()
            );
            let operator = fixture
                .context(Principal::Device {
                    id: vessel_proto::DeviceId::random(),
                    scopes: vec![Scope::Operate],
                })
                .await;
            let applied = phase(
                "human Operate does not imply a Read requirement",
                service.apply_session_options(&operator.connection, request(parent.id())),
            )
            .await
            .unwrap();
            assert!(applied.failure().is_none());
            operator.finish().await;
            agent.finish().await;
            owner.finish().await;
            fixture.finish().await;
        }),
    )
    .await
    .unwrap();
}
