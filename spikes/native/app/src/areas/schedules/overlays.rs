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
    let key = area.ui.selected.as_ref()?;
    let row = area.data.rows.iter().find(|row| row.key == *key)?;
    let items: Vec<_> = row
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
    let actions: Vec<_> = row.menu.iter().map(|entry| entry.action).collect();
    let key = key.clone();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.schedules.ui.menu,
        chose: Rc::new(move |shell, index, window, cx| {
            if let Some(action) = actions.get(index) {
                if *action == Action::Delete {
                    shell.schedule_delete_prompt(&key, window, cx);
                } else {
                    shell.schedule_act(&key, *action);
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
fn close(shell: &mut Shell, _window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.areas.schedules.ui.deleting = None;
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
    let enabled = area
        .data
        .page
        .row(&prompt.key)
        .is_some_and(|row| row.actions().contains(&Action::Delete));
    let sizing = ControlSizing::from_compact(touch);
    let cancel = kit::button_with_kind_at(
        "schedules-keep",
        shell.words.schedules().cancel(),
        ButtonKind::Secondary,
        ButtonState::Enabled,
        sizing,
        tokens,
    )
    .on_click(cx.listener(|shell, _, window, cx| close(shell, window, cx)));
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
        let key = prompt.key.clone();
        delete = delete.on_click(cx.listener(move |shell, _, _, cx| {
            shell.areas.schedules.ui.deleting = None;
            shell.schedule_act(&key, Action::Delete);
            cx.notify();
        }));
    }
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
        close,
        cx,
    ))
}
