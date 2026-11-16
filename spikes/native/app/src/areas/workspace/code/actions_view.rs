//! A tree entry's actions as drawn: the row a name is typed in, the menu
//! (Paseo's `file-actions-menu.tsx` groups, a hairline between them, the
//! delete in its own group in the destructive tone), the delete's confirm
//! in the kit's dialog, and the machine's refusal over the tree.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    MenuHandlers, MenuItem, Tokens, Type, TypeExt as _, breakpoint, button, glyph, menu,
};

use super::actions::{Action, EditKind, group, items_for};
use crate::shell::Shell;

/// The row being typed, at `depth`: the entry's glyph and the field.
pub(super) fn edit_row(
    shell: &Shell,
    depth: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(editing) = shell.areas.workspace.code.files.actions.editing.as_ref() else {
        return div().into_any_element();
    };
    let neutral = tokens.neutral;
    let folder = matches!(editing.kind, EditKind::NewFolder(_));
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .py(px(4.0))
        .pl(px(super::tree::indented(depth)))
        .pr(px(8.0))
        .child(
            div()
                .size(px(16.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(glyph(
                    if folder { Glyph::Folder } else { Glyph::File },
                    16.0,
                    neutral.foreground_muted,
                )),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .h(px(24.0))
                .px(px(6.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(neutral.border_accent)
                .bg(neutral.surface1)
                .child(editing.field.input(tokens, cx)),
        )
        .child(
            div()
                .id("code-edit-cancel")
                .size(px(24.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .hover(move |style| style.bg(neutral.surface2))
                .child(glyph(Glyph::Close, 12.0, neutral.foreground_muted))
                .on_click(cx.listener(|this, _event, _window, cx| this.cancel_edit(cx))),
        )
        .into_any_element()
}

/// The menu and the delete's dialog, over everything, when open.
pub(super) fn overlays(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let actions = &shell.areas.workspace.code.files.actions;
    let labels = shell.words().labels();
    let narrow = breakpoint::phone(window);
    let mut drawn = Vec::new();
    if let Some(target) = &actions.target {
        let chosen = items_for(target);
        let mut last_group = None;
        let items: Vec<MenuItem> = chosen
            .iter()
            .map(|action| {
                let label = match action {
                    Action::OpenFile => labels.open_file,
                    Action::CollapseFolder => labels.collapse_folder,
                    Action::NewFile => labels.new_file,
                    Action::NewFolder => labels.new_folder,
                    Action::CopyPath => labels.copy_path,
                    Action::CopyRelativePath => labels.copy_relative_path,
                    Action::AddToChat => labels.add_to_chat,
                    Action::Rename => labels.rename,
                    Action::Duplicate => labels.duplicate,
                    Action::Delete => labels.delete,
                };
                let item = if *action == Action::Delete {
                    MenuItem::danger(label)
                } else {
                    MenuItem::new(label)
                };
                let starts = last_group.is_some_and(|last| last != group(*action));
                last_group = Some(group(*action));
                if starts && *action != Action::Delete {
                    item.grouped()
                } else {
                    item
                }
            })
            .collect();
        let handlers = MenuHandlers {
            state: |shell: &mut Shell| &mut shell.areas.workspace.code.files.actions.menu,
            chose: std::rc::Rc::new(move |shell: &mut Shell, index, window, cx| {
                if let Some(action) = chosen.get(index).copied() {
                    shell.file_action(action, window, cx);
                }
            }),
        };
        drawn.extend(menu(
            "code-file-actions",
            &actions.menu,
            &items,
            &handlers,
            tokens,
            narrow,
            cx,
        ));
    }
    drawn.extend(delete_dialog(shell, tokens, window, narrow, cx));
    drawn
}

/// The delete's confirm (Paseo's words), when one waits.
fn delete_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let actions = &shell.areas.workspace.code.files.actions;
    let labels = shell.words().labels();
    let (path, folder) = actions.deleting.as_ref()?;
    let name = path.file_name().unwrap_or_default().to_owned();
    let content = DialogContent {
        title: if *folder {
            labels.delete_folder_title
        } else {
            labels.delete_file_title
        }
        .into(),
        body: div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(labels.delete_message)
            .child(
                div()
                    .type_(Type::Mono)
                    .text_color(tokens.neutral.foreground)
                    .child(name),
            )
            .into_any_element(),
        footer: vec![
            button("code-delete-cancel", labels.cancel, false, tokens)
                .on_click(cx.listener(|this, _event, _window, cx| {
                    this.areas.workspace.code.files.actions.deleting = None;
                    cx.notify();
                }))
                .into_any_element(),
            vessel_app_kit::button_with_kind(
                "code-delete-confirm",
                labels.delete,
                vessel_app_kit::ButtonKind::Destructive,
                vessel_app_kit::ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(|this, _event, _window, cx| this.confirm_delete(cx)))
            .into_any_element(),
        ],
        size: DialogSize::Normal,
    };
    Some(dialog(
        &actions.dialog,
        content,
        tokens,
        window,
        narrow,
        |shell: &mut Shell, _window, cx| {
            shell.areas.workspace.code.files.actions.deleting = None;
            cx.notify();
        },
        cx,
    ))
}

/// The machine's refusal of the last change, over the tree.
pub(super) fn refused_note(shell: &Shell, tokens: &Tokens) -> Option<Div> {
    let reason = shell.areas.workspace.code.files.actions.refused.as_ref()?;
    Some(super::viewer::failed(
        shell.words().labels().change_refused,
        reason,
        12.0,
        tokens,
    ))
}
