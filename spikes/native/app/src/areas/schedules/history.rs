//! Read-only run history through existing rows/dialog; no alternate session
//! opener. Actual session navigation waits for the shared context-bound owner.
use crate::shell::Shell;
use std::sync::Arc;
use vessel_app_kit::{
    self as kit, ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _,
    dialog::{DialogContent, DialogSize},
    theme::spacing,
    ui::{
        AnyElement, Context, Div, IntoElement as _, ParentElement as _, SharedString,
        StatefulInteractiveElement as _, Styled as _, Window, div, px,
    },
};
use vessel_app_link::Command;
use vessel_app_view::schedules::{
    history::{presentation::Run, reader::Visit},
    page::{Body, Intent},
};

pub(super) fn open(
    shell: &mut Shell,
    intent: &Intent,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let area = &mut shell.areas.schedules;
    if !intent.is_current(&area.data.page)
        || area.ui.deleting.is_some()
        || area.ui.editor.is_mounted()
    {
        return;
    }
    let Some(row) = area.data.page.row(intent.key()) else {
        return;
    };
    let Some(request) = area.data.history.open(&row) else {
        let _changed = shell.schedules_changed(cx);
        cx.notify();
        return;
    };
    if area.ui.history.take().is_some() {
        area.ui.history_dialog.dismissed(window, cx);
    }
    area.ui.history = area.data.history.visit();
    area.ui.history_dialog.focus(window, cx);
    shell.command(Command::Schedules(request.into()));
    let _changed = shell.schedules_changed(cx);
    cx.notify();
}
fn close(shell: &mut Shell, visit: Visit, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let area = &mut shell.areas.schedules;
    if area.ui.history != Some(visit) || !area.data.history.close(visit) {
        return;
    }
    area.ui.history = None;
    area.ui.history_dialog.dismissed(window, cx);
    let _changed = shell.schedules_changed(cx);
    cx.notify();
}
fn refresh(shell: &mut Shell, visit: Visit, cx: &mut Context<'_, Shell>) {
    if shell.areas.schedules.ui.history != Some(visit) {
        return;
    }
    if let Some(request) = shell.areas.schedules.data.history.retry(visit) {
        shell.command(Command::Schedules(request.into()));
    }
    let _changed = shell.schedules_changed(cx);
    cx.notify();
}
pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.schedules;
    let visit = area.ui.history?;
    if area.data.history.visit() != Some(visit) {
        return None;
    }
    let title = match &area.data.history_cache {
        Ok(Some(page)) => SharedString::from(Arc::clone(&page.title)),
        Ok(None) | Err(_) => shell.words.schedules().history_heading().into(),
    };
    let close_button = kit::button_with_kind_at(
        "history-close",
        shell.words.schedules().history_close(),
        ButtonKind::Secondary,
        ButtonState::Enabled,
        ControlSizing::from_compact(touch),
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| close(shell, visit, window, cx)));
    let enabled = area.data.history.can_retry(visit);
    let mut refresh_button = kit::button_with_kind_at(
        "history-refresh",
        shell.words.schedules().history_refresh(),
        ButtonKind::Ghost,
        if enabled {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        ControlSizing::from_compact(touch),
        tokens,
    );
    if enabled {
        refresh_button =
            refresh_button.on_click(cx.listener(move |shell, _, _, cx| refresh(shell, visit, cx)));
    }
    Some(kit::dialog(
        &area.ui.history_dialog,
        DialogContent {
            title,
            body: body(shell, tokens, touch),
            footer: vec![
                close_button.into_any_element(),
                refresh_button.into_any_element(),
            ],
            size: DialogSize::Normal,
        },
        tokens,
        window,
        touch,
        move |shell, window, cx| close(shell, visit, window, cx),
        cx,
    ))
}
fn body(shell: &Shell, tokens: &Tokens, touch: bool) -> AnyElement {
    let cache = &shell.areas.schedules.data.history_cache;
    let page = match cache {
        Ok(Some(page)) => page,
        Ok(None) => return div().into_any_element(),
        Err(notice) => return super::draw::row_notice(notice, tokens).into_any_element(),
    };
    match page.body {
        Body::Loading => div()
            .flex()
            .items_center()
            .justify_center()
            .p(px(spacing::S6))
            .child(kit::loading::activity_indicator(
                "history-loading",
                kit::loading::Size::Large,
                tokens.neutral.foreground_muted,
            ))
            .into_any_element(),
        Body::Empty => div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .whitespace_normal()
            .child(shell.words.schedules().history_empty())
            .into_any_element(),
        Body::Failed => div()
            .children(
                page.notice
                    .as_ref()
                    .map(|notice| super::draw::row_notice(notice, tokens)),
            )
            .into_any_element(),
        Body::Content => {
            let mut card = kit::settings::card(tokens);
            for (index, run) in page.runs.iter().enumerate() {
                card = card.child(row(run, index == 0, tokens, touch));
            }
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S3))
                .children(
                    page.notice
                        .as_ref()
                        .map(|notice| super::draw::row_notice(notice, tokens)),
                )
                .child(card)
                .into_any_element()
        }
    }
}
fn row(run: &Run, first: bool, tokens: &Tokens, touch: bool) -> Div {
    let content = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .child(
            div()
                .type_(Type::Body)
                .whitespace_normal()
                .child(SharedString::from(Arc::clone(&run.heading))),
        )
        .children(run.reason.as_ref().map(|reason| {
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .whitespace_normal()
                .child(SharedString::from(Arc::clone(reason)))
        }));
    let badge = kit::badge::badge(
        SharedString::from(Arc::clone(&run.outcome)),
        crate::badge::variant(run.variant),
        kit::badge::Size::ExtraSmall,
        None,
        tokens,
    );
    kit::settings::row_with_content(
        content.into_any_element(),
        Some(badge.into_any_element()),
        first,
        touch,
        tokens,
    )
}
