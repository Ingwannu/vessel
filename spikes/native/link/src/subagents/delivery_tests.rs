//! Pending feature refusal still travels through the actual guarded actor path.
use crate::{Command, Event, testing::actor::Actor};
use vessel_app_view::{
    devices::Tickets,
    subagents::{SubagentAsk as Ask, SubagentReply as Reply, Target, TrackProblem},
};
use vessel_proto::SessionId;
#[tokio::test]
async fn queued_subagent_read_refusal_is_revalidated_before_model_application() {
    let mut actor = Actor::new("fixture").await;
    let target = Target::new(actor.context(), SessionId::random());
    let ticket = Tickets::default().next_ticket().unwrap();
    actor
        .dispatch(Command::Subagents(Ask::Lookup {
            target: target.clone(),
            ticket,
        }))
        .await;
    let Event::Subagents(answer) = actor.next_event().await else {
        panic!("subagents")
    };
    actor.retire().await;
    let Reply::LookedUp {
        target: original,
        ticket: original_ticket,
        result,
    } = answer.into_reply()
    else {
        panic!("lookup")
    };
    assert_eq!(original, target);
    assert_eq!(original_ticket, ticket);
    assert_eq!(result, Err(TrackProblem::IdentityChanged));
}
#[tokio::test]
async fn preflight_refusal_needs_no_fabricated_guard() {
    let mut actor = Actor::new("fixture").await;
    let target = Target::new(actor.context(), SessionId::random());
    let ticket = Tickets::default().next_ticket().unwrap();
    actor.retire().await;
    let _retired = actor.next_event().await;
    actor
        .dispatch(Command::Subagents(Ask::Lookup { target, ticket }))
        .await;
    let Event::Subagents(answer) = actor.next_event().await else {
        panic!("subagents")
    };
    assert!(matches!(
        answer.into_reply(),
        Reply::LookedUp {
            result: Err(TrackProblem::IdentityChanged),
            ..
        }
    ));
}
