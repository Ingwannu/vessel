//! The frozen unit Actor still drives the real dispatcher and fallible peer.
use super::{actor::Actor, within};
use crate::{
    Command, Event,
    devices::{Ask, Outcome},
};
use vessel_app_view::devices::Tickets;
use vessel_proto::{Call, DeviceSelf, Outcome as WireOutcome, Reply, Scope};

#[tokio::test]
async fn the_frozen_actor_uses_the_shared_framed_core_and_original_retirement() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let context = actor.context();
        let ticket = Tickets::default().next_ticket().unwrap();
        actor.dispatch(Command::Devices { context: context.clone(), ticket, ask: Ask::Authority }).await;
        let request = actor.request().await;
        assert!(matches!(request.call, Call::GetDeviceSelf(_)));
        let facts = DeviceSelf { device: None, scopes: vec![Scope::Admin] };
        request.answer.send(WireOutcome::Ok(Reply::GetDeviceSelf(facts.clone()))).unwrap();
        let Event::Devices(answer) = actor.next_event().await else { panic!("expected actual Devices answer"); };
        assert_eq!(answer.context, context);
        assert!(matches!(answer.outcome, Outcome::Authority(actual, Ok(reported)) if actual == ticket && reported == facts));
        actor.retire().await;
        actor.close_peer().await;
    }).await;
}
