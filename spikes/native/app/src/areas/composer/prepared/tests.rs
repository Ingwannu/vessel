//! Real shell state with no link, credentials, file I/O or selected timeline.
use super::*;
use crate::{areas::composer::attach::Incoming, shell::language_tests};
use gpui::TestAppContext;
use vessel_app_view::{
    Attention, Phrase,
    composer::tray::{ItemState, Target},
};
use vessel_proto::{AgentAbilities, AttachmentLimits, NodeId, SessionId};

pub(in crate::areas::composer) fn connection(
    node: NodeId,
) -> vessel_app_view::connection::ConnectionContext {
    vessel_app_view::connection::ConnectionEpochs::default()
        .admit(
            vessel_app_view::connection::MachineKey::from_book("fixture"),
            node,
        )
        .unwrap()
}

pub(in crate::areas::composer) fn card(machine: &str) -> Card {
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
        detail: vessel_app_view::board::CardDetail::default(),
    }
}
#[gpui::test]
fn stale_ticket_and_wrong_machine_cannot_consume_prepared_files(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let node = NodeId::random();
        assert_eq!(
            shell
                .prepare_draft_input_on(
                    Ticket::new(1),
                    vessel_proto::timeline::ClientMessageId::random(),
                    crate::areas::composer::prepared::tests::connection(node),
                    "text"
                )
                .unwrap(),
            None
        );
        shell.composer_add(
            DraftKey::New,
            vec![Incoming::pasted("image/png", vec![1, 2, 3])],
            cx,
        );
        let prepared = shell
            .prepare_draft_input_on(
                Ticket::new(2),
                vessel_proto::timeline::ClientMessageId::random(),
                crate::areas::composer::prepared::tests::connection(node),
                "first",
            )
            .unwrap()
            .unwrap();
        assert_eq!(prepared.ticket(), Ticket::new(2));
        assert_eq!(
            shell.prepare_draft_input_on(
                Ticket::new(3),
                vessel_proto::timeline::ClientMessageId::random(),
                crate::areas::composer::prepared::tests::connection(node),
                "second"
            ),
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
        let node = NodeId::random();
        let ticket = Ticket::new(1);
        shell.composer_add(
            DraftKey::New,
            vec![Incoming::pasted("image/png", vec![1])],
            cx,
        );
        let _prepared = shell
            .prepare_draft_input_on(
                ticket,
                vessel_proto::timeline::ClientMessageId::random(),
                crate::areas::composer::prepared::tests::connection(node),
                "captured",
            )
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
            connection: connection(node),
            ticket,
            expected_node: node,
            machine: vessel_app_view::machine_book::Name::parse("fixture").unwrap(),
            session: card.session,
        };
        let target = Target {
            abilities: Some(AgentAbilities::default()),
            limits: AttachmentLimits::DEFAULT,
        };
        let mut stale = ask.clone();
        stale.ticket = Ticket::new(99);
        shell.composer_capabilities(&stale, Ok(target), cx);
        assert!(shell.areas.composer.capabilities.blocked(card.session));
        shell.composer_capabilities(&ask, Ok(target), cx);
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

pub(in crate::areas::composer) fn history(
    text: &str,
) -> vessel_app_view::composer::context::ChatHistory {
    use vessel_app_view::composer::context::{
        ChatHistory, ContextId, ContextText, ContextTitle, Payload,
    };
    ChatHistory::new(Payload {
        id: ContextId::parse("history").unwrap(),
        title: ContextTitle::parse("Previous conversation").unwrap(),
        text: ContextText::parse(text).unwrap(),
        url: None,
    })
}

#[gpui::test]
fn context_only_creation_freezes_history_and_rejects_only_the_matching_ticket(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        assert!(!shell.draft_has_payload(cx));
        shell
            .composer_seed_history(DraftKey::New, history("first"), window, cx)
            .unwrap();
        assert!(shell.draft_has_payload(cx));
        assert!(shell.draft_has_files());
        let ticket = Ticket::new(11);
        let _prepared = shell
            .prepare_draft_input_on(
                ticket,
                vessel_proto::timeline::ClientMessageId::random(),
                crate::areas::composer::prepared::tests::connection(NodeId::random()),
                "",
            )
            .unwrap()
            .unwrap();
        shell
            .areas
            .composer
            .contexts
            .tray(DraftKey::New)
            .reattach(history("newer").into_attachment())
            .unwrap();
        shell.rejected_draft_files(Ticket::new(10), cx);
        assert!(shell.areas.composer.starting.borrow().is_some());
        let card = card("fixture");
        shell.started_draft_files(ticket, &card, cx);
        assert!(shell.areas.session.open.is_none());
        let pending = &shell.areas.composer.sends[&card.session];
        assert_eq!(
            pending
                .context
                .as_ref()
                .unwrap()
                .attachments()
                .map(vessel_app_view::composer::context::ContextAttachment::text)
                .collect::<Vec<_>>(),
            ["first"]
        );
        assert_eq!(
            shell
                .areas
                .composer
                .contexts
                .get(DraftKey::New)
                .unwrap()
                .visible()
                .map(vessel_app_view::composer::context::ContextAttachment::text)
                .collect::<Vec<_>>(),
            ["newer"]
        );
    });
}

#[gpui::test]
fn seeding_refuses_whitespace_and_suppressed_source_context(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        use vessel_app_view::composer::context::ContextProblem;
        shell.areas.draft.composer.set_text(" ", window, cx);
        assert!(shell.draft_has_payload(cx));
        assert_eq!(
            shell.composer_seed_history(DraftKey::New, history("history"), window, cx),
            Err(ContextProblem::Busy)
        );
        shell.areas.draft.composer.clear(window, cx);
        let item = history("existing").into_attachment();
        let tray = shell.areas.composer.contexts.tray(DraftKey::New);
        tray.add(item.clone()).unwrap();
        tray.remove(&item);
        assert!(!shell.composer_has_context(DraftKey::New));
        assert!(shell.draft_has_payload(cx));
        assert_eq!(
            shell.composer_seed_history(DraftKey::New, history("replacement"), window, cx),
            Err(ContextProblem::Busy)
        );
    });
}
