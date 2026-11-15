//! Application menus reuse the kit popup; native window controls retain OS behavior.

use crate::shell::Shell;
use gpui::{Action, FocusHandle};
use std::rc::Rc;
use vessel_app_kit::menu::{MenuHandlers, MenuItem, menu};
use vessel_app_kit::theme::spacing::S1;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind};
use vessel_app_view::shortcuts::desktop::DesktopAction;

#[derive(Debug, Clone, Copy)]
pub(super) enum Group {
    All,
    File,
    Edit,
    View,
    Window,
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
    Changelog,
}

struct Row {
    item: Item,
    view: MenuItem,
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
                            ButtonState::Enabled,
                            tokens,
                        )
                        .occlude()
                        .on_click(cx.listener(
                            move |shell, event: &gpui::ClickEvent, window, cx| {
                                let area = &mut shell.areas.desktop;
                                area.focus = window.focused(cx);
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

fn rows(shell: &Shell, group: Group) -> Vec<Row> {
    let w = shell.words.desktop_shell();
    let mut rows = Vec::new();
    let mut push = |item, label: &str, starts_group| {
        let mut view = MenuItem::new(label.to_owned());
        view.starts_group = starts_group;
        if let Item::Desktop(
            DesktopAction::ZoomIn | DesktopAction::ZoomOut | DesktopAction::ActualSize,
        ) = item
        {
            view.disabled = shell.areas.shortcuts.page.recording();
        }
        rows.push(Row { item, view });
    };
    if matches!(group, Group::All | Group::File) {
        push(
            Item::Desktop(DesktopAction::NewWindow),
            w.new_window(),
            false,
        );
        push(
            Item::Desktop(DesktopAction::CloseWindow),
            w.close_window(),
            false,
        );
        push(Item::Quit, w.quit(), true);
    }
    if matches!(group, Group::All | Group::Edit) {
        for (item, label) in [
            (Edit::Undo, w.undo()),
            (Edit::Redo, w.redo()),
            (Edit::Cut, w.cut()),
            (Edit::Copy, w.copy()),
            (Edit::Paste, w.paste()),
            (Edit::SelectAll, w.select_all()),
        ] {
            push(
                Item::Edit(item),
                label,
                matches!(item, Edit::Undo | Edit::Cut | Edit::SelectAll),
            );
        }
    }
    if matches!(group, Group::All | Group::View) {
        push(Item::Desktop(DesktopAction::ZoomIn), w.zoom_in(), true);
        push(Item::Desktop(DesktopAction::ZoomOut), w.zoom_out(), false);
        push(
            Item::Desktop(DesktopAction::ActualSize),
            w.actual_size(),
            false,
        );
        push(Item::Reload, w.reload(), true);
        push(Item::ForceReload, w.force_reload(), false);
        push(
            Item::Desktop(DesktopAction::Fullscreen),
            w.fullscreen(),
            true,
        );
        push(Item::Changelog, w.whats_new(), true);
    }
    if matches!(group, Group::All | Group::Window) {
        push(Item::Minimize, w.minimize(), true);
        push(Item::Maximize, w.maximize(), false);
    }
    rows
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
    let rows = rows(shell, shell.areas.desktop.group);
    let items: Vec<_> = rows.iter().map(|row| row.view.clone()).collect();
    menu(
        "desktop-application-menu",
        &shell.areas.desktop.menu,
        &items,
        &MenuHandlers {
            state: |shell: &mut Shell| &mut shell.areas.desktop.menu,
            chose: Rc::new(move |shell, index, window, cx| {
                if let Some(row) = rows.get(index) {
                    let focus = shell.areas.desktop.focus.take();
                    invoke(shell, row.item, focus, window, cx);
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
    focus: Option<FocusHandle>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if let Some(focus) = focus {
        focus.focus(window, cx);
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
        Item::Changelog => shell.open_changelog(window, cx),
    }
}

fn edit_action(edit: Edit) -> Box<dyn Action> {
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
