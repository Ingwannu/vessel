//! Dispatch actual retained production controls after a new visit opens.
use super::*;
use gpui::{AppContext as _, Entity, Render, TestAppContext};
use std::{cell::Cell, rc::Rc};
use vessel_app_kit::ui::{IntoElement, div};
use vessel_app_kit::{Scheme, SchemeChoice, Tokens};
use vessel_app_link::Link;
use vessel_app_view::{
    connection::ConnectionEpochs,
    devices::MachineKey,
    history::{Ask, Entry, Listed, Reply, Scope},
};
use vessel_proto::{HarnessKind, HarnessSessionId, NodeId};

struct Probe {
    shell: Entity<Shell>,
    renders: Rc<Cell<usize>>,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.renders.set(self.renders.get().saturating_add(1));
        self.shell.update(cx, |shell, cx| {
            render(shell, &Tokens::of(Scheme::Dark), window, cx)
                .unwrap_or_else(|| div().into_any_element())
        })
    }
}
fn scope() -> Scope {
    let context = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    Scope {
        machine: context.machine().clone(),
        context,
        harness: HarnessKind::Codex.into(),
        profile: None,
        cwd: None,
    }
}
fn loaded(shell: &mut Shell, scope: &Scope) {
    let area = &mut shell.areas.import;
    assert!(area.choose(scope.clone()));
    let Ask::List { ticket, .. } = area.model.refresh().unwrap().unwrap() else {
        panic!("list")
    };
    assert!(area.model.answer(Reply::Listed {
        scope: scope.clone(),
        ticket,
        result: Ok(Listed {
            entries: vec![Entry {
                id: HarnessSessionId::new("native"),
                cwd: "/work".into(),
                title: None,
                updated_at: None,
                in_vessel: None
            }],
            next: None,
            node: scope.context.node()
        })
    }));
    area.model.toggle(&HarnessSessionId::new("native"));
    area.reproject();
}
#[path = "tests/callbacks.rs"]
mod callbacks;

#[gpui::test]
fn successful_import_receipt_never_defers_alias_navigation_or_clears_the_active_draft(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
    let original = scope();
    let (link, mut commands) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let current = vessel_proto::SessionId::random();
    shell.update_in(cx, |shell, window, cx| {
        shell.open_import(window, cx);
        loaded(shell, &original);
        shell.areas.draft.folder = "/draft/keep".into();
        shell
            .areas
            .session
            .composer
            .set_text("keep session input", window, cx);
        shell
            .areas
            .draft
            .composer
            .set_text("keep new draft input", window, cx);
        shell.areas.conversation.open("fixture", current);
        let Ask::Import(batch) = shell.areas.import.model.import().unwrap().unwrap() else {
            panic!("import")
        };
        let entry = &batch.entries[0];
        let receipt = vessel_proto::SessionInfo::new(
            vessel_proto::SessionId::random(),
            entry.spec(&batch.scope),
            chrono::DateTime::UNIX_EPOCH,
        );
        shell.history_apply_reply(
            Reply::Imported {
                scope: batch.scope,
                ticket: batch.ticket,
                outcomes: vec![vessel_app_view::history::Outcome {
                    id: entry.id.clone(),
                    result: Ok(Box::new(receipt)),
                }],
            },
            cx,
        );
        assert!(shell.areas.import.open, "the receipt stays available");
        assert_eq!(shell.areas.import.model.outcomes().len(), 1);
        // A same-node replacement and then an alias rebound cannot receive a
        // deferred command: receipt installation has no navigation registration.
        for node in [original.context.node(), NodeId::random()] {
            let mut epochs = ConnectionEpochs::default();
            let _old = epochs
                .admit(original.machine.clone(), original.context.node())
                .unwrap();
            let next = epochs.admit(original.machine.clone(), node).unwrap();
            assert_ne!(next.epoch(), original.context.epoch());
            let mut machine =
                vessel_app_view::Machine::unreachable("fixture".into(), "fixture offline");
            machine.node = Some(next.node());
            machine.connection_epoch = Some(next.epoch());
            shell.fleet.machines = vec![machine];
        }
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, cx| {
        assert_eq!(shell.areas.session.composer.text(cx), "keep session input");
        assert_eq!(shell.areas.draft.composer.text(cx), "keep new draft input");
        assert_eq!(shell.areas.draft.folder, "/draft/keep");
        assert_eq!(
            shell.areas.conversation.shown.as_ref().unwrap().session,
            current
        );
        assert!(shell.areas.import.open);
    });
    while let Ok(command) = commands.try_recv() {
        assert!(
            !matches!(
                command,
                vessel_app_link::Command::Open { .. } | vessel_app_link::Command::Reload
            ),
            "no Open, follow or reload may escape"
        );
    }
}
