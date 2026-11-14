//! Scope and agent refusals through real secure device connections.

use super::*;

pub(crate) async fn device(
    owner: &NodeClient,
    node: &vessel_node::Node,
    scopes: Vec<Scope>,
) -> NodeClient {
    let mut offer = owner
        .invite_with_scopes(Some("side access test".to_owned()), 600, scopes)
        .await
        .unwrap()
        .link
        .decode()
        .unwrap();
    let direct = crate::secure::direct_route(node);
    offer.direct = vec![direct.clone()];
    let (paired, _route) = vessel_client::secure::pair_over(
        &crate::secure::dialer(),
        &offer,
        "side access test",
        vessel_client::secure::RouteTimeouts::default(),
    )
    .await
    .unwrap();
    crate::secure::device_via(crate::secure::dialer(), &paired, &[&direct], &[])
        .connect(crate::support::client())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn both_scopes_are_required_and_agents_cannot_manufacture_side_links() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(crate::agents::vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut parent = crate::agents::structured(&owner, &fixture).await;
        let (_, socket, auth) = crate::agents::credential(&owner, &mut parent).await;
        let agent = crate::agents::as_agent(&socket, &auth).await;
        assert_eq!(
            code(agent.open_side_question(parent.id).await),
            ErrorCode::Forbidden
        );
        let mut spec = structured_spec(&fixture, None);
        spec.side_question_of = Some(parent.id);
        assert_eq!(code(agent.create_session(spec).await), ErrorCode::Forbidden);
        let side = owner.open_side_question(parent.id).await.unwrap();
        let _side_stream = ready(&owner, side.id).await;
        for scopes in [vec![Scope::Read], vec![Scope::Operate]] {
            let caller = device(&owner, &node, scopes).await;
            assert_eq!(
                code(caller.open_side_question(parent.id).await),
                ErrorCode::Forbidden
            );
            assert_eq!(
                code(
                    caller
                        .prompt_side_question(side.id, "no access".parse().unwrap())
                        .await
                ),
                ErrorCode::Forbidden
            );
            assert_eq!(
                code(caller.reset_side_question(parent.id, side.id).await),
                ErrorCode::Forbidden
            );
        }
        let reader = device(&owner, &node, vec![Scope::Read]).await;
        assert_eq!(reader.session(side.id).await.unwrap().id, side.id);
        let _followed = following(&reader, side.id, EventSeq::ZERO).await;
        let operator = device(&owner, &node, vec![Scope::Read, Scope::Operate]).await;
        assert_eq!(
            operator.open_side_question(parent.id).await.unwrap().id,
            side.id
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saved_and_scheduled_inputs_cannot_bypass_the_side_prompt_boundary() {
    within(async {
        use vessel_proto::automation::{
            AfterRun, Cadence, CatchUp, CreatePrompt, ScheduleDefinition, ScheduleTarget,
        };
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let parent = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap();
        let _parent_stream = ready(&client, parent.id).await;
        let side = client.open_side_question(parent.id).await.unwrap();
        let _side_stream = ready(&client, side.id).await;
        let mut spec = structured_spec(&fixture, None);
        spec.side_question_of = Some(parent.id);
        assert_eq!(
            code(
                client
                    .create_prompt(CreatePrompt {
                        name: "side template".parse().unwrap(),
                        text: "question".parse().unwrap(),
                        project: None,
                        session: Some(Box::new(spec.clone())),
                    })
                    .await
            ),
            ErrorCode::InvalidRequest
        );
        for (target, expected) in [
            (
                ScheduleTarget::NewSession {
                    spec: Box::new(spec),
                    after: AfterRun::default(),
                },
                ErrorCode::InvalidRequest,
            ),
            (
                ScheduleTarget::Session { session: side.id },
                ErrorCode::Unsupported,
            ),
        ] {
            assert_eq!(
                code(
                    client
                        .create_schedule(
                            ScheduleDefinition {
                                name: None,
                                cadence: Cadence::Every {
                                    seconds: 60_u32.try_into().unwrap(),
                                    anchor: None
                                },
                                target,
                                prompt: "question".parse().unwrap(),
                                catch_up: CatchUp::default(),
                                max_runs: None,
                                ends_at: None,
                            },
                            true,
                        )
                        .await
                ),
                expected
            );
        }
        node.stop().await.unwrap();
    })
    .await;
}
