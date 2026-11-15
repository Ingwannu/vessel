//! Retained drafts accept events only from the real editor that owns them.
//!
//! [Decision Log]
//! - Purpose: queued edits from a replaced editor must not change a new draft.
//! - Constraint: the shared field now supplies engine-owned FieldId callbacks.
//! - Choice: use that existing API and replace the editor on a new edit target;
//!   retain the same editor through async/language changes for selection/copy.
//! - Cost: a new form target gets a new input identity, not a second renderer.

use crate::shell::Shell;
use vessel_app_kit::TextField;
use vessel_app_kit::field::FieldId;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::labels::{EditAccess, HostState};

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
