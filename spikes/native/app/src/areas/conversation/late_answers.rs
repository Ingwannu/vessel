//! An act's answer reaches the conversation's state only when it is for
//! the session shown (track V1's review of the lead's head 64b5a3d).

use gpui::TestAppContext;
use vessel_app_link::{ActAnswer, ActFailed, ActKind, Acted};
use vessel_proto::{ErrorCode, SessionId};

use super::ConversationArea;
use super::area::Followup;

fn answer(session: SessionId, kind: ActKind, result: Result<Acted, ActFailed>) -> ActAnswer {
    ActAnswer {
        terminal: None,
        machine: "laptop".to_owned(),
        session,
        kind,
        result,
    }
}

fn refused(message: &str) -> Result<Acted, ActFailed> {
    Err(ActFailed {
        code: Some(ErrorCode::InvalidRequest),
        message: message.to_owned(),
    })
}

fn unreachable(message: &str) -> Result<Acted, ActFailed> {
    Err(ActFailed {
        code: None,
        message: message.to_owned(),
    })
}

fn area(cx: &mut TestAppContext) -> ConversationArea {
    cx.update(|cx| {
        vessel_app_kit::init(
            cx,
            vessel_app_kit::SchemeChoice::Fixed(vessel_app_kit::Scheme::Dark),
        )
        .expect("the faces register");
        ConversationArea::new(cx)
    })
}

/// Asked from A, answered after the person moved to B: success, Done
/// and both failures leave B as it was, and nothing is read again.
#[gpui::test]
fn a_late_answer_for_another_session_changes_nothing(cx: &mut TestAppContext) {
    let mut area = area(cx);
    let (a, b) = (SessionId::random(), SessionId::random());
    area.open("laptop", a);
    area.open("laptop", b);
    area.lost = Some("b's own".to_owned());
    for late in [
        answer(a, ActKind::Permit, Ok(Acted::Done)),
        answer(a, ActKind::Rename, refused("a's refusal")),
        answer(a, ActKind::Info, unreachable("a's transport")),
    ] {
        assert_eq!(area.answered(late), Followup::None);
        assert_eq!(area.refused, None);
        assert_eq!(area.lost.as_deref(), Some("b's own"));
    }
    // The same session on another machine is another session.
    let mut elsewhere = answer(b, ActKind::Rename, refused("other machine"));
    elsewhere.machine = "build-server".to_owned();
    assert_eq!(area.answered(elsewhere), Followup::None);
    assert_eq!(area.refused, None);
}

/// The shown session's machine was lost: a late answer does not say the
/// records flow again, so the lost line stays; the shown session's own
/// refusal and re-read still apply.
#[gpui::test]
fn a_late_answer_after_the_connection_was_lost_keeps_it_lost(cx: &mut TestAppContext) {
    let mut area = area(cx);
    let a = SessionId::random();
    area.open("laptop", a);
    area.lost = Some("closed".to_owned());
    assert_eq!(
        area.answered(answer(a, ActKind::Info, Ok(Acted::Done))),
        Followup::None
    );
    assert_eq!(area.lost.as_deref(), Some("closed"));
    assert_eq!(
        area.answered(answer(a, ActKind::Permit, Ok(Acted::Done))),
        Followup::ReadWaiting
    );
    assert_eq!(
        area.answered(answer(a, ActKind::Rename, refused("taken"))),
        Followup::None
    );
    assert_eq!(
        area.refused.as_ref().map(|(_, why)| why.as_str()),
        Some("taken")
    );
    assert_eq!(area.lost.as_deref(), Some("closed"));
}

/// The link's queue full: the read the opened session asked comes
/// back, and the session's row settles refused in the app's words,
/// not lost (no machine was asked).
#[gpui::test]
fn an_act_the_full_link_gave_back_settles_refused(cx: &mut TestAppContext) {
    const WORDS: &str = "Could not send. Try again.";
    let mut area = area(cx);
    let session = SessionId::random();
    area.open("laptop", session);
    let (link, _inbox) = vessel_app_link::Link::detached(1).expect("a runtime");
    let read = || vessel_app_link::Command::Act {
        machine: "laptop".to_owned(),
        session,
        act: vessel_app_link::Act::Info,
    };
    assert!(link.submit(read()).is_ok(), "the first fills the queue");
    let unsent = link.submit(read()).expect_err("a full queue gives it back");
    let crate::unsent::Settle::Answer(event) = crate::unsent::settle(unsent, WORDS) else {
        panic!("an act settles as its answer");
    };
    let vessel_app_link::Event::Acted(answer) = *event else {
        panic!("an act settles as an act's answer");
    };
    assert_eq!(area.answered(answer), Followup::None);
    assert_eq!(area.refused, Some((ActKind::Info, WORDS.to_owned())));
    assert_eq!(area.lost, None);
}
