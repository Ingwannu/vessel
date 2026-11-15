//! Application menus reuse the kit popup; native window controls retain OS behavior.

use crate::shell::Shell;
use gpui::Action;
use std::rc::Rc;
use vessel_app_kit::menu::{MenuHandlers, MenuItem, menu};
use vessel_app_kit::theme::spacing::S1;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind};
use vessel_app_view::shortcuts::desktop::DesktopAction;

#[cfg(test)]
mod recipient_tests;
mod rows;
#[cfg(test)]
mod tests;
pub(super) use rows::rows;

#[derive(Debug, Clone, Copy)]
pub(super) enum Group {
    All,
    File,
    Edit,
    View,
    Window,
    Context,
    Help,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Edit {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Item {
    Desktop(DesktopAction),
    Edit(Edit),
    Reload,
    ForceReload,
    Minimize,
    Maximize,
    Quit,
    Help(vessel_app_view::desktop::HelpAction),
}

pub(super) struct Row {
    pub(super) item: Item,
    pub(super) view: MenuItem,
}

pub(super) fn open_context(
    shell: &mut Shell,
    position: gpui::Point<gpui::Pixels>,
    edit: vessel_app_view::desktop::EditContext,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if !shell.desktop_commands_admitted() {
        return;
    }
    let area = &mut shell.areas.desktop;
    area.menu.close_and_restore(window, cx);
    area.menu_cycle = Rc::new(());
    area.edit_recipient = window.focused(cx).map(|focus| focus.downgrade());
    area.group = Group::Context;
    area.edit = edit;
    area.menu.open(position, window, cx);
    cx.notify();
}

pub(crate) fn trigger(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if cfg!(target_os = "macos") {
        return None;
    }
    let words = shell.words.desktop_shell();
    let buttons = if crate::areas::draft::compact(window) {
        vec![(Group::All, words.menu())]
    } else {
        vec![
            (Group::File, words.file()),
            (Group::Edit, words.edit()),
            (Group::View, words.view()),
            (Group::Window, words.window()),
            (Group::Help, words.help()),
        ]
    };
    Some(
        div()
            .flex()
            .gap(px(S1))
            .children(
                buttons
                    .into_iter()
                    .enumerate()
                    .map(|(index, (group, label))| {
                        button_with_kind(
                            ("desktop-menu", index),
                            label.to_owned(),
                            ButtonKind::Ghost,
                            if shell.desktop_commands_admitted() {
                                ButtonState::Enabled
                            } else {
                                ButtonState::Disabled
                            },
                            tokens,
                        )
                        .occlude()
                        .on_click(cx.listener(
                            move |shell, event: &gpui::ClickEvent, window, cx| {
                                if !shell.desktop_commands_admitted() {
                                    return;
                                }
                                let area = &mut shell.areas.desktop;
                                area.menu.close_and_restore(window, cx);
                                area.menu_cycle = Rc::new(());
                                area.edit_recipient =
                                    window.focused(cx).map(|focus| focus.downgrade());
                                area.group = if crate::areas::draft::compact(window) {
                                    Group::All
                                } else {
                                    group
                                };
                                area.menu.open(event.position(), window, cx);
                                cx.notify();
                            },
                        ))
                        .into_any_element()
                    }),
            )
            .into_any_element(),
    )
}

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !shell.areas.desktop.menu.is_open() {
        return None;
    }
    let cycle = Rc::clone(&shell.areas.desktop.menu_cycle);
    let recipient = shell.areas.desktop.edit_recipient.clone();
    let rows = rows(shell, shell.areas.desktop.group);
    let items: Vec<_> = rows.iter().map(|row| row.view.clone()).collect();
    menu(
        "desktop-application-menu",
        &shell.areas.desktop.menu,
        &items,
        &MenuHandlers {
            state: |shell: &mut Shell| &mut shell.areas.desktop.menu,
            chose: Rc::new(move |shell, index, window, cx| {
                if !Rc::ptr_eq(&cycle, &shell.areas.desktop.menu_cycle) {
                    return;
                }
                if let Some(row) = rows.get(index) {
                    // The shared menu alone owns return-focus. If a newer input
                    // won that race, its preserved focus must not redirect an
                    // older popup's edit operation into a different recipient.
                    if matches!(row.item, Item::Edit(_))
                        && !recipient
                            .as_ref()
                            .and_then(gpui::WeakFocusHandle::upgrade)
                            .is_some_and(|focus| focus.is_focused(window))
                    {
                        return;
                    }
                    invoke(shell, row.item, window, cx);
                }
            }),
        },
        tokens,
        crate::areas::draft::compact(window),
        cx,
    )
}

pub(super) fn invoke(
    shell: &mut Shell,
    item: Item,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if !shell.desktop_commands_admitted() {
        return;
    }
    match item {
        Item::Desktop(action) => {
            let _handled = shell.run_desktop_shortcut(action, window, cx);
        }
        Item::Edit(edit) => window.dispatch_action(edit_action(edit), cx),
        Item::Reload | Item::ForceReload => {
            shell.command(vessel_app_link::Command::Reload);
            window.refresh();
        }
        Item::Minimize => window.minimize_window(),
        Item::Maximize => window.zoom_window(),
        Item::Quit => crate::desktop::request_quit(cx),
        Item::Help(action) => shell.run_desktop_help(action, window, cx),
    }
}

pub(super) fn edit_action(edit: Edit) -> Box<dyn Action> {
    // GPUI's action registry is an open toolkit boundary; the app's closed
    // edit vocabulary stays exhaustive before entering that boundary.
    use vessel_app_kit::editing;
    match edit {
        Edit::Undo => Box::new(editing::Undo),
        Edit::Redo => Box::new(editing::Redo),
        Edit::Cut => Box::new(editing::Cut),
        Edit::Copy => Box::new(editing::Copy),
        Edit::Paste => Box::new(editing::Paste),
        Edit::SelectAll => Box::new(editing::SelectAll),
    }
}
