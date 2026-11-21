//! The link hands the connected machine its own report through the real
//! framed actor: its session focus or elsewhere, and `Active` only for
//! input it has not heard of.

use crate::testing::{actor::Actor, within};
use vessel_app_view::presence::{Presence, Shown};
use vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL;
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
        assert!(presence.show(Some(("fixture", session.into())), 0));
        actor.presence(presence.snapshot(0));
        let first = reported(&mut actor).await;
        assert_eq!(first.focus, PushFocus::Session { session });
        assert_eq!(first.activity, PushActivity::Active);
        assert_eq!(first.idle_age, None);

        // Another machine's session shown, with no new input since: this
        // machine is shown elsewhere, and hears nothing new.
        assert!(presence.show(Some(("other", session.into())), 0));
        actor.presence(presence.snapshot(15_000));
        let second = reported(&mut actor).await;
        assert_eq!(second.focus, PushFocus::Elsewhere);
        assert_eq!(second.activity, PushActivity::Unchanged);

        // Hidden, then input: background, and active once.
        assert!(presence.visibility(false, 20_000));
        presence.input(21_000);
        actor.presence(presence.snapshot(30_000));
        let third = reported(&mut actor).await;
        assert_eq!(third.focus, PushFocus::Background);
        assert_eq!(third.activity, PushActivity::Active);

        actor.retire().await;
        actor.close_peer().await;
    })
    .await;
}

#[tokio::test]
async fn a_shown_terminal_is_terminal_focus_only_to_a_machine_that_admits_claims() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let terminal = vessel_proto::TerminalId::random();
        let mut presence = Presence::new(0, true);
        assert!(presence.show(Some(("fixture", Shown::Terminal(terminal))), 0));
        actor.presence(presence.snapshot(0));
        let report = reported(&mut actor).await;
        // A report carrying `Terminal` focus is versioned to ADR 0089's
        // constant: while it is unassigned the machine hears `Elsewhere`,
        // not the route's session (Paseo's focus target is the terminal).
        let expected = if ATTENTION_NOTIFICATIONS_PROTOCOL.is_some() {
            PushFocus::Terminal { terminal }
        } else {
            PushFocus::Elsewhere
        };
        assert_eq!(report.focus, expected);
        actor.retire().await;
        actor.close_peer().await;
    })
    .await;
}
