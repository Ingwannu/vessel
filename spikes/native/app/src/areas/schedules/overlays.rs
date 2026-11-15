//! Shared action menu and destructive confirmation. Every entry comes from
//! View's wired action list; RPCs start only from an explicit activation.
use crate::shell::Shell;
use std::{rc::Rc, sync::Arc};
use vessel_app_kit::{
    self as kit, ButtonKind, ButtonState, ControlSizing, Tokens,
    dialog::{DialogContent, DialogSize},
    menu::{Content, MenuHandlers, MenuItem},
    ui::{
        AnyElement, Context, IntoElement as _, ParentElement as _, SharedString,
        StatefulInteractiveElement as _, Window, div, px,
    },
};
use vessel_app_view::schedules::{Action, page::EntryState};
const ACTION_MENU_WIDTH: f32 = 220.0;
#[derive(Clone, Copy)]
enum Choice {
    Write(Action),
    History,
}
fn glyph(action: Action) -> kit::assets::Glyph {
    use kit::assets::Glyph;
    match action {
        Action::Edit => Glyph::Pencil,
        Action::Pause => Glyph::Pause,
        Action::Resume => Glyph::Play,
        Action::RunNow => Glyph::RotateCw,
        Action::Delete => Glyph::Trash2,
    }
}
pub(super) fn menu(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.schedules;
    let selected = area.ui.selected.as_ref()?;
    if !selected.is_current(&area.data.page) {
        return None;
    }
    let row = area
        .data
        .rows
        .iter()
        .find(|row| row.key == *selected.key())?;
    let mut items: Vec<_> = row
        .menu
        .iter()
        .map(|entry| {
            let label = SharedString::from(Arc::clone(&entry.label));
            let mut item = if entry.action == Action::Delete {
                MenuItem::danger(label)
            } else {
                MenuItem::new(label)
            }
            .with_leading(glyph(entry.action));
            item.starts_group = entry.action == Action::Delete;
            match entry.state {
                EntryState::Enabled => {}
                EntryState::Disabled => item.disabled = true,
                EntryState::Pending => {
                    if let Some(pending) = &entry.pending {
                        item = item.pending(SharedString::from(Arc::clone(pending)));
                    } else {
                        item.disabled = true;
                    }
                }
            }
            item
        })
        .collect();
    let mut actions: Vec<_> = row
        .menu
        .iter()
        .map(|entry| Choice::Write(entry.action))
        .collect();
    if let Some(label) = &row.history {
        items.push(
            MenuItem::new(SharedString::from(Arc::clone(label)))
                .with_leading(kit::assets::Glyph::CalendarClock),
        );
        actions.push(Choice::History);
    }
    let selected = Rc::clone(selected);
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.schedules.ui.menu,
        chose: Rc::new(move |shell, index, window, cx| {
            if !shell
                .areas
                .schedules
                .ui
                .selected
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &selected))
            {
                return;
            }
            if let Some(action) = actions.get(index) {
                match *action {
                    Choice::Write(Action::Edit) => {
                        super::editor::open_edit(shell, &selected, window, cx)
                    }
                    Choice::Write(Action::Delete) => {
                        shell.schedule_delete_prompt(&selected, window, cx)
                    }
                    Choice::Write(action) => shell.schedule_act(&selected, action, cx),
                    Choice::History => super::history::open(shell, &selected, window, cx),
                }
            }
            cx.notify();
        }),
    };
    kit::menu::menu_with(
        Content::new("schedules-actions", &items).with_width(px(ACTION_MENU_WIDTH)),
        &area.ui.menu,
        &handlers,
        tokens,
        touch,
        cx,
    )
}
fn close(
    expected: &Rc<super::DeletePrompt>,
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let ui = &mut shell.areas.schedules.ui;
    if !ui
        .deleting
        .as_ref()
        .is_some_and(|current| Rc::ptr_eq(current, expected))
    {
        return;
    }
    ui.deleting = None;
    ui.dialog.dismissed(window, cx);
    cx.notify();
}
pub(super) fn deletion(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.schedules;
    let prompt = area.ui.deleting.as_ref()?;
    let enabled = prompt.intent.allows(&area.data.page, Action::Delete);
    let cancel_prompt = Rc::clone(prompt);
    let sizing = ControlSizing::from_compact(touch);
    let cancel = kit::button_with_kind_at(
        "schedules-keep",
        shell.words.schedules().cancel(),
        ButtonKind::Secondary,
        ButtonState::Enabled,
        sizing,
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| close(&cancel_prompt, shell, window, cx)));
    let mut delete = kit::button_with_kind_at(
        "schedules-delete",
        shell.words.schedules().delete_confirm_action(),
        ButtonKind::Destructive,
        if enabled {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        sizing,
        tokens,
    );
    if enabled {
        let confirmed = Rc::clone(prompt);
        delete = delete.on_click(cx.listener(move |shell: &mut Shell, _, window, cx| {
            let area = &shell.areas.schedules;
            if !area
                .ui
                .deleting
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &confirmed))
                || !confirmed.intent.allows(&area.data.page, Action::Delete)
            {
                return;
            }
            // Paseo confirms first, then dispatches the mutation. A refused
            // stale callback neither starts work nor acknowledges dismissal.
            close(&confirmed, shell, window, cx);
            shell.schedule_act(&confirmed.intent, Action::Delete, cx);
            cx.notify();
        }));
    }
    let dismissed = Rc::clone(prompt);
    Some(kit::dialog(
        &area.ui.dialog,
        DialogContent {
            title: prompt.title.clone(),
            body: div().child(prompt.body.clone()).into_any_element(),
            footer: vec![cancel.into_any_element(), delete.into_any_element()],
            size: DialogSize::Normal,
        },
        tokens,
        window,
        touch,
        move |shell, window, cx| close(&dismissed, shell, window, cx),
        cx,
    ))
}
