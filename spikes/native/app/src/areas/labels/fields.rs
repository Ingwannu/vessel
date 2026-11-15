//! Retained drafts accept events only from the real editor that owns them.
//!
//! [Decision Log]
//! - Purpose: queued edits from a replaced editor must not change a new draft.
//! - Constraint: the shared field now supplies engine-owned FieldId callbacks.
//! - Choice: use that existing API and replace the editor on a new edit target;
//!   retain the same editor through async/language changes for selection/copy.
//!   Search gets a fresh field per opening and accepts input only in its visible
//!   listing phase, so an old queued edit cannot undo close/reopen query reset.
//! - Cost: a new form target gets a new input identity, not a second renderer.

use crate::shell::Shell;
use vessel_app_kit::TextField;
use vessel_app_kit::field::FieldId;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::labels::{EditAccess, HostState, ManagerState};

pub(super) fn search(
    placeholder: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> TextField {
    TextField::with_source(
        placeholder,
        |_shell: &mut Shell, _source, _text, _window, _cx| {},
        search_changed,
        window,
        cx,
    )
}

fn search_changed(shell: &mut Shell, source: FieldId, text: String, cx: &mut Context<'_, Shell>) {
    let state = &mut shell.areas.labels;
    if state.opening.is_none()
        || state.search.id() != source
        || !matches!(state.manager.state(), ManagerState::Listing)
    {
        return;
    }
    state.manager.search(&text);
    cx.notify();
}

pub(super) fn manager(
    placeholder: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> TextField {
    TextField::with_source(
        placeholder,
        |_shell: &mut Shell, _source, _text, _window, _cx| {},
        changed,
        window,
        cx,
    )
}

fn changed(shell: &mut Shell, source: FieldId, text: String, cx: &mut Context<'_, Shell>) {
    let state = &mut shell.areas.labels;
    if state.name.id() != source {
        return;
    }
    let availability = state
        .selected_host()
        .map_or(HostState::Offline, |host| host.state);
    if state.manager.edit_access(availability) != EditAccess::Editable {
        return;
    }
    if let Some(draft) = state.manager.draft_mut() {
        draft.set_text(&text);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use vessel_app_view::{
        connection::{ConnectionEpochs, MachineKey},
        labels::{Catalog, Color, Host, Label},
        machine_book::Name,
    };
    use vessel_proto::NodeId;

    #[gpui::test]
    fn closed_manager_search_cannot_deliver_a_queued_change_into_its_successor(
        cx: &mut TestAppContext,
    ) {
        crate::shell::language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, window, cx| {
            shell.labels_open(window, cx);
            let source = shell.areas.labels.search.id();
            search_changed(shell, source, "old query".to_owned(), cx);
            assert_eq!(shell.areas.labels.manager.query(), "old query");
            cx.defer_in(window, move |shell, _window, cx| {
                search_changed(shell, source, "queued old query".to_owned(), cx);
            });
            let state = &shell.areas.labels;
            let dismissal = super::super::surface::Dismiss::capture(
                state.opening.unwrap(),
                state.manager.state(),
            );
            shell.labels_close(dismissal, window, cx);
            search_changed(shell, source, "hidden query".to_owned(), cx);
            assert!(shell.areas.labels.manager.query().is_empty());
            shell.labels_open(window, cx);
            assert_ne!(source, shell.areas.labels.search.id());
        });
        cx.run_until_parked();
        shell.update(cx, |shell, cx| {
            assert!(shell.areas.labels.manager.query().is_empty());
            let source = shell.areas.labels.search.id();
            search_changed(shell, source, "fresh query".to_owned(), cx);
            assert_eq!(shell.areas.labels.manager.query(), "fresh query");
        });
    }

    #[gpui::test]
    fn replaced_manager_field_refuses_its_queued_change(cx: &mut TestAppContext) {
        crate::shell::language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, window, cx| {
            let context = ConnectionEpochs::default()
                .admit(MachineKey::from_book("fixture"), NodeId::random())
                .unwrap();
            let label = Label {
                name: "Ready".parse().unwrap(),
                color: Color::Violet,
            };
            let state = &mut shell.areas.labels;
            state.route = Some(Name::parse("fixture").unwrap());
            state.hosts = vec![Host {
                key: context.node(),
                route: Name::parse("fixture").unwrap(),
                connection: Some(context.clone()),
                state: HostState::Online,
                catalog: Catalog::snapshot([label.clone()]),
            }];
            state.manager.select(context.node()).unwrap();
            state.manager.edit(label).unwrap();
            let source = state.name.id();
            cx.defer_in(window, move |shell, _window, cx| {
                changed(shell, source, "stale".to_owned(), cx);
            });
            state.name = manager("", window, cx);
            assert_ne!(source, state.name.id());
        });
        cx.run_until_parked();
        shell.update(cx, |shell, cx| {
            assert_eq!(
                shell.areas.labels.manager.state().draft().unwrap().text(),
                "Ready"
            );
            let source = shell.areas.labels.name.id();
            changed(shell, source, "fresh".to_owned(), cx);
            assert_eq!(
                shell.areas.labels.manager.state().draft().unwrap().text(),
                "fresh"
            );
        });
    }
}
