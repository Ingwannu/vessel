//! Real shell state with no link, credentials, file I/O or selected timeline.
use super::*;
use crate::{areas::composer::attach::Incoming, shell::language_tests};
use gpui::TestAppContext;
use vessel_app_view::{
    Attention, Phrase,
    composer::tray::{ItemState, Target},
};
use vessel_proto::{AgentAbilities, AttachmentLimits, SessionId};

fn card(machine: &str) -> Card {
    Card {
        session: SessionId::random(),
        structured: true,
        live: true,
        title: "Created".into(),
        machine: machine.into(),
        agent: "Fixture".into(),
        lane: None,
        phrase: Phrase::Starting,
        attention: Attention::AtRest,
        at: chrono::Utc::now(),
        detail: Default::default(),
    }
}
#[gpui::test]
fn stale_ticket_and_wrong_machine_cannot_consume_prepared_files(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        assert_eq!(
            shell
                .prepare_draft_files(Ticket::new(1), "fixture", "text")
                .unwrap(),
            None
        );
        shell.composer_add(
            DraftKey::New,
            vec![Incoming::pasted("image/png", vec![1, 2, 3])],
            cx,
        );
        let prepared = shell
            .prepare_draft_files(Ticket::new(2), "fixture", "first")
            .unwrap()
            .unwrap();
        assert_eq!(prepared.ticket(), Ticket::new(2));
        assert_eq!(
            shell.prepare_draft_files(Ticket::new(3), "fixture", "second"),
            Err(PrepareProblem::Busy)
        );
        shell.rejected_draft_files(Ticket::new(1), cx);
        shell.started_draft_files(Ticket::new(1), &card("fixture"), cx);
        shell.started_draft_files(Ticket::new(2), &card("other"), cx);
        assert!(shell.areas.composer.starting.borrow().is_some());
        assert!(shell.composer_has_files(DraftKey::New));
        shell.rejected_draft_files(Ticket::new(2), cx);
        assert!(shell.areas.composer.starting.borrow().is_none());
        assert!(shell.composer_has_files(DraftKey::New));
    });
}
#[gpui::test]
fn background_completion_moves_only_captured_files_and_reads_its_own_capabilities(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let ticket = Ticket::new(1);
        shell.composer_add(
            DraftKey::New,
            vec![Incoming::pasted("image/png", vec![1])],
            cx,
        );
        let _prepared = shell
            .prepare_draft_files(ticket, "fixture", "captured")
            .unwrap()
            .unwrap();
        shell.composer_add(
            DraftKey::New,
            vec![Incoming::pasted("image/png", vec![2])],
            cx,
        );
        let card = card("fixture");
        shell.started_draft_files(ticket, &card, cx);
        assert!(
            shell.areas.session.open.is_none(),
            "background delivery must not follow the created session"
        );
        assert_eq!(shell.route, crate::routes::Route::Draft);
        assert_eq!(
            shell
                .areas
                .composer
                .trays
                .get(DraftKey::New)
                .unwrap()
                .items()
                .len(),
            1
        );
        let key = DraftKey::Session {
            session: card.session,
        };
        assert_eq!(
            shell.areas.composer.trays.get(key).unwrap().items().len(),
            1
        );
        assert!(shell.areas.composer.capabilities.blocked(card.session));
        let ask = vessel_app_link::attachments::CapabilityAsk {
            ticket,
            machine: vessel_app_view::machine_book::Name::parse("fixture").unwrap(),
            session: card.session,
        };
        let target = Target {
            abilities: Some(AgentAbilities::default()),
            limits: AttachmentLimits::DEFAULT,
        };
        let mut stale = ask.clone();
        stale.ticket = Ticket::new(99);
        shell.composer_capabilities(stale, Ok(target), cx);
        assert!(shell.areas.composer.capabilities.blocked(card.session));
        shell.composer_capabilities(ask, Ok(target), cx);
        assert!(!shell.areas.composer.capabilities.blocked(card.session));
        assert!(
            matches!(
                shell.areas.composer.trays.get(key).unwrap().items()[0].state,
                ItemState::Refused(_)
            ),
            "the created session's no-image capability must apply without a selected timeline"
        );
        assert!(shell.areas.session.open.is_none());
    });
}
