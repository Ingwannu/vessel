//! The link hands the connected machine its own report through the real
//! framed actor: its session focus or elsewhere, and `Active` only for
//! input it has not heard of.

use crate::Command;
use crate::testing::{actor::Actor, within};
use vessel_app_view::presence::Presence;
use vessel_proto::push::{PushActivity, PushFocus, ReportPushPresence};
use vessel_proto::{Ack, Call, Outcome as WireOutcome, Reply, SessionId};

async fn reported(actor: &mut Actor) -> ReportPushPresence {
    let request = actor.request().await;
    let Call::ReportPushPresence(report) = request.call else {
        panic!("expected push.presence, got {:?}", request.call);
    };
    request
        .answer
        .send(WireOutcome::Ok(Reply::ReportPushPresence(Ack {})))
        .unwrap();
    report
}

#[tokio::test]
async fn a_snapshot_reaches_the_machine_with_its_focus_and_new_input_only() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let session = SessionId::random();
        let mut presence = Presence::new(0, true);
        assert!(presence.show(Some(("fixture", session)), 0));
        actor
            .dispatch(Command::Presence(presence.snapshot(0)))
            .await;
        let first = reported(&mut actor).await;
        assert_eq!(first.focus, PushFocus::Session { session });
        assert_eq!(first.activity, PushActivity::Active);
        assert_eq!(first.idle_age, None);

        // Another machine's session shown, with no new input since: this
        // machine is shown elsewhere, and hears nothing new.
        assert!(presence.show(Some(("other", session)), 0));
        actor
            .dispatch(Command::Presence(presence.snapshot(15_000)))
            .await;
        let second = reported(&mut actor).await;
        assert_eq!(second.focus, PushFocus::Elsewhere);
        assert_eq!(second.activity, PushActivity::Unchanged);

        // Hidden, then input: background, and active once.
        assert!(presence.visibility(false, 20_000));
        presence.input(21_000);
        actor
            .dispatch(Command::Presence(presence.snapshot(30_000)))
            .await;
        let third = reported(&mut actor).await;
        assert_eq!(third.focus, PushFocus::Background);
        assert_eq!(third.activity, PushActivity::Active);

        actor.retire().await;
        actor.close_peer().await;
    })
    .await;
}
