//! No real provider: preflight and recipient refusal against an isolated node.
use crate::bridge::{op_reply, reply};
use crate::support::{Source, Tab, TestNode, Ui, within};
use vessel_proto::timeline::{ClientMessageId, SubmissionDelivery, SubmitPrompt};
use vessel_proto::{Call, ContentBlock, GetSession, SessionId};
use vessel_web::wire::{CallOutcome, Failure, Op, OpOutcome, OpResult, SubmitCheck};

fn request() -> SubmitPrompt {
    SubmitPrompt {
        session: SessionId::random(),
        message: ClientMessageId::random(),
        content: vec![ContentBlock::Text {
            text: "synthetic".to_owned(),
        }],
        delivery: SubmissionDelivery::Start,
        native_command: None,
    }
}

async fn check(tab: &mut Tab, expected_node: vessel_proto::NodeId) -> SubmitCheck {
    let id = tab
        .op(Op::CheckSubmit {
            node: "box".parse().unwrap(),
            expected_node,
            request: Box::new(request()),
        })
        .await;
    match op_reply(tab, id).await {
        OpOutcome::Ok(OpResult::CheckSubmit { check }) => check,
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn unassigned_submit_is_refused_locally_and_a_changed_recipient_never_dispatches() {
    within(Box::pin(async {
        let node = TestNode::start().await;
        let ui = Ui::start(Source::default().with("box", node.address(), &node.token())).await;
        let mut tab = ui.tab().await;
        // A missing session would produce Remote(NotFound) if dispatched. The
        // SDK instead refuses the unassigned feature before sending any call.
        assert!(matches!(
            check(&mut tab, node.id()).await,
            SubmitCheck::Unsupported {
                failure: Failure::Failed { .. }
            }
        ));
        let wrong = vessel_proto::NodeId::random();
        assert!(matches!(
            check(&mut tab, wrong).await,
            SubmitCheck::Refused {
                failure: Failure::Refused { .. }
            }
        ));
        let call = Call::GetSession(GetSession {
            session: SessionId::random(),
        });
        let id = tab.prepared_call("box", wrong, &call).await;
        assert!(matches!(
            reply(&mut tab, id).await,
            CallOutcome::Err(Failure::Refused { .. })
        ));
        // The same ordinary call does reach the node with the correct identity.
        let id = tab.prepared_call("box", node.id(), &call).await;
        assert!(matches!(
            reply(&mut tab, id).await,
            CallOutcome::Err(Failure::Remote { .. })
        ));
    }))
    .await;
}
