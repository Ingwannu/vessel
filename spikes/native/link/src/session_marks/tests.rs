//! Real frames and original actor admission, never a fake delivery capability.
use super::*;
use crate::{
    Command, Event,
    testing::{actor::Actor, peer, within},
};
use vessel_app_view::{
    Card, Machine, Reach,
    connection::ConnectionContext,
    session_marks::{CapturedSessionTarget, Requests},
};
use vessel_proto::{Call, Mark, Outcome, SessionId};
fn ask(context: &ConnectionContext, session: SessionId, mark: Mark) -> Ask {
    let card = Card::from_session(context.machine().as_str(), &peer::info(session));
    let machine = Machine {
        name: context.machine().as_str().to_owned(),
        node: Some(context.node()),
        connection_epoch: Some(context.epoch()),
        sessions: vec![card.clone()],
        reach: Reach::Online,
        agents: vec![],
        protocol: None,
        version: None,
    };
    let target = CapturedSessionTarget::capture(&machine, &card).unwrap();
    Requests::default().begin(target, mark).unwrap()
}
async fn answer(actor: &mut Actor) -> Answer {
    let Event::SessionMarked(answer) = actor.next_event().await else {
        panic!("directory mark event")
    };
    answer
}
#[tokio::test]
async fn actual_mark_ack_preserves_original_intent_and_exact_peer_metadata() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let session = SessionId::random();
        let ask = ask(&actor.context(), session, Mark::Pin);
        actor.dispatch(Command::SessionMark(ask.clone())).await;
        let request = actor.request().await;
        assert!(matches!(request.call, Call::MarkSession(mark) if mark.session == session && mark.mark == Mark::Pin));
        let mut info = peer::info(session);
        info.marks.pinned = true;
        request.answer.send(Outcome::Ok(vessel_proto::Reply::MarkSession(info.clone()))).unwrap();
        let (original, result) = answer(&mut actor).await.into_reply().into_parts();
        assert_eq!(original, ask);
        assert_eq!(*result.unwrap(), info);
        actor.close_peer().await;
    }).await;
}
#[tokio::test]
async fn wrong_session_or_unsatisfied_postcondition_cannot_be_a_success() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let session = SessionId::random();
        for returned in [SessionId::random(), session] {
            actor
                .dispatch(Command::SessionMark(ask(
                    &actor.context(),
                    session,
                    Mark::Pin,
                )))
                .await;
            let request = actor.request().await;
            let mut info = peer::info(returned);
            // Wrong identity has an otherwise valid postcondition; the second
            // frame isolates a correct identity with an invalid postcondition.
            info.marks.pinned = returned != session;
            request
                .answer
                .send(Outcome::Ok(vessel_proto::Reply::MarkSession(info)))
                .unwrap();
            assert!(matches!(
                answer(&mut actor).await.into_reply().into_parts().1,
                Err(Failure::InvalidReply)
            ));
        }
        actor.close_peer().await;
    })
    .await;
}
#[tokio::test]
async fn queued_real_mark_ack_cannot_install_after_admission_retirement() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let session = SessionId::random();
        actor
            .dispatch(Command::SessionMark(ask(
                &actor.context(),
                session,
                Mark::Unpin,
            )))
            .await;
        let request = actor.request().await;
        request
            .answer
            .send(Outcome::Ok(vessel_proto::Reply::MarkSession(peer::info(
                session,
            ))))
            .unwrap();
        let queued = answer(&mut actor).await;
        actor.retire().await;
        assert!(matches!(
            queued.into_reply().into_parts().1,
            Err(Failure::Uncertain)
        ));
        actor.close_peer().await;
    })
    .await;
}
#[tokio::test]
async fn physical_close_after_queued_ack_is_still_uncertain() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let session = SessionId::random();
        actor
            .dispatch(Command::SessionMark(ask(
                &actor.context(),
                session,
                Mark::Unpin,
            )))
            .await;
        let request = actor.request().await;
        request
            .answer
            .send(Outcome::Ok(vessel_proto::Reply::MarkSession(peer::info(
                session,
            ))))
            .unwrap();
        let queued = answer(&mut actor).await;
        actor.close_peer().await;
        assert!(matches!(
            queued.into_reply().into_parts().1,
            Err(Failure::Uncertain)
        ));
    })
    .await;
}

#[tokio::test]
async fn archive_and_unarchive_are_only_original_directory_marks() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let session = SessionId::random();
        for mark in [Mark::Archive, Mark::Unarchive] {
            let original = ask(&actor.context(), session, mark);
            actor.dispatch(Command::SessionMark(original.clone())).await;
            let request = actor.request().await;
            assert!(matches!(request.call, Call::MarkSession(request) if request.session == session && request.mark == mark));
            let mut info = peer::info(session);
            info.status = vessel_proto::SessionStatus::Exited { code: None };
            info.marks.archived_at = (mark == Mark::Archive).then_some(chrono::DateTime::UNIX_EPOCH);
            request.answer.send(Outcome::Ok(vessel_proto::Reply::MarkSession(info.clone()))).unwrap();
            let (retained, result) = answer(&mut actor).await.into_reply().into_parts();
            assert_eq!(retained, original);
            assert_eq!(*result.unwrap(), info);
        }
        // The worker issued only the two explicitly observed MarkSession calls;
        // no create/resume operation or resident completion was requested.
        actor.close_peer().await;
    }).await;
}

mod replacement;
