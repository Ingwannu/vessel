//! Person and agent origins through an actual fleet hop and its intersected
//! device grant; pure Principal tests alone do not prove the envelope path.

use super::*;
use vessel_proto::side_questions::{OpenSideQuestion, PromptSideQuestion, ResetSideQuestion};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forwarded_side_calls_preserve_scope_intersections_and_refuse_agent_origins() {
    within(async {
        let relay = crate::secure::Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let member = crate::across::member_granting(
            &relay,
            &home,
            "side-caller",
            vec![Scope::Sync, Scope::Relay, Scope::Read, Scope::Operate],
            vec![Scope::Read, Scope::Operate],
        )
        .await;
        let parent = home
            .owner
            .create_session(structured_spec(&home.fixture, None))
            .await
            .unwrap();
        let _parent_stream = ready(&home.owner, parent.id).await;
        let before = read_all(&home.owner, parent.id).await;
        let side = member
            .owner
            .call_on(home.id(), OpenSideQuestion { session: parent.id })
            .await
            .unwrap();
        let mut stream = ready(&home.owner, side.id).await;
        let _answer = member
            .owner
            .call_on(
                home.id(),
                PromptSideQuestion {
                    session: side.id,
                    question: "What is happening?".parse().unwrap(),
                },
            )
            .await
            .unwrap();
        let _turn = permission(&home.owner, side.id, &mut stream).await;
        assert_eq!(read_all(&home.owner, parent.id).await, before);
        // These callers are local owners, so the source permits them.
        // The missing scope is in the receiving home's member grant;
        // refusal therefore exercises the delegated intersection itself.
        for (label, granted) in [
            ("read-grant", Scope::Read),
            ("operate-grant", Scope::Operate),
        ] {
            let restricted = crate::across::member_granting(
                &relay,
                &home,
                label,
                vec![Scope::Sync, Scope::Relay, granted],
                vec![Scope::Read, Scope::Operate],
            )
            .await;
            forbidden_calls(&restricted.owner, home.id(), parent.id, side.id).await;
            restricted.node.stop().await.unwrap();
        }
        let mut origin = crate::agents::structured(&member.owner, &member.fixture).await;
        let (_, socket, auth) = crate::agents::credential(&member.owner, &mut origin).await;
        let agent = crate::agents::as_agent(&socket, &auth).await;
        forbidden_calls(&agent, home.id(), parent.id, side.id).await;
        let replacement = member
            .owner
            .call_on(
                home.id(),
                ResetSideQuestion {
                    session: parent.id,
                    side: side.id,
                },
            )
            .await
            .unwrap();
        assert_ne!(replacement.id, side.id);
        assert_eq!(replacement.spec.side_question_of, Some(parent.id));
        member.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

/// Each operation is checked independently; a forwarded origin must not
/// gain a prompt or reset path after its open was refused.
async fn forbidden_calls(
    caller: &NodeClient,
    target: vessel_proto::NodeId,
    parent: SessionId,
    side: SessionId,
) {
    assert_eq!(
        code(
            caller
                .call_on(target, OpenSideQuestion { session: parent })
                .await
        ),
        ErrorCode::Forbidden
    );
    assert_eq!(
        code(
            caller
                .call_on(
                    target,
                    PromptSideQuestion {
                        session: side,
                        question: "no access".parse().unwrap(),
                    }
                )
                .await
        ),
        ErrorCode::Forbidden
    );
    assert_eq!(
        code(
            caller
                .call_on(
                    target,
                    ResetSideQuestion {
                        session: parent,
                        side
                    }
                )
                .await
        ),
        ErrorCode::Forbidden
    );
}
