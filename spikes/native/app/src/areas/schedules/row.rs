//! `ScheduleRow` source parts and edit activation; retained clicks carry the
//! exact admitted intent, never a later row-index/alias lookup.
use crate::shell::Shell;
use kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, div, px,
};
use std::sync::Arc;
use vessel_app_kit::{self as kit, Tokens, Type, TypeExt as _, theme::spacing};
use vessel_app_view::schedules::presentation::PreparedRow;

#[cfg(test)]
mod tests;

pub(super) fn render(
    shell: &Shell,
    row: &PreparedRow,
    index: usize,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let main = div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(spacing::S3))
        .flex_1()
        .min_w_0()
        .child(mark(row, tokens))
        .child(words(row, tokens));
    let content = div().flex_1().min_w_0().child(main).children(
        row.notice
            .as_ref()
            .map(|notice| super::draw::row_notice(notice, tokens)),
    );
    let trailing = trailing(shell, row, tokens, touch, cx);
    let element = kit::settings::row_with_content(
        content.into_any_element(),
        Some(trailing),
        index == 0,
        touch,
        tokens,
    )
    .id(("schedules-row", index));
    if let Some(intent) = &row.intent
        && super::editor::editable(shell, intent)
    {
        let intent = intent.clone();
        let words = shell.words.schedules();
        let label = words.form_edit_row(&row.title)
            .unwrap_or_else(|_| words.form_edit_title().to_owned());
        element.role(gpui::Role::Button).aria_label(label)
            .cursor_pointer()
            .hover({
                let wash = tokens.neutral.interaction_highlight;
                move |style| style.bg(wash)
            })
            .on_click(cx.listener(move |shell, _, window, cx| shell.schedule_edit_open(&intent, window, cx)))
    } else {
        element
    }
}

fn target(row: &PreparedRow, tokens: &Tokens) -> Div {
    div()
        .mt(px(spacing::S1))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .truncate()
        .child(SharedString::from(Arc::clone(&row.target_label)))
}
fn words(row: &PreparedRow, tokens: &Tokens) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .truncate()
                .child(SharedString::from(Arc::clone(&row.title))),
        )
        .child(target(row, tokens))
        .child(
            div()
                .mt(px(spacing::S1))
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .truncate()
                .child(SharedString::from(Arc::clone(&row.metadata))),
        )
}
fn mark(row: &PreparedRow, tokens: &Tokens) -> Div {
    div()
        .w(px(kit::agent_mark::Size::Sixteen.points()))
        .h(px(kit::theme::icon::LG))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .children(row.agent.as_ref().map(|agent| {
            crate::agent_mark::mark_muted(
                Some(agent),
                kit::agent_mark::Size::Sixteen,
                kit::agent_mark::Space::Compact,
                tokens,
            )
        }))
}

fn trailing(
    shell: &Shell,
    row: &PreparedRow,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let badge = kit::badge::badge(
        vessel_app_view::schedules::locale::badge(row.status, shell.words),
        crate::badge::variant(row.status.badge()),
        kit::badge::Size::Small,
        None,
        tokens,
    );
    let mut controls = div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .flex_none()
        .child(badge);
    let ui = &shell.areas.schedules.ui;
    if row.has_menu()
        && let Some(intent) = &row.intent
        && let Some(control) = ui.anchors.get(&row.key)
    {
        let expanded = ui.menu.is_open()
            && ui
                .selected
                .as_ref()
                .is_some_and(|selected| selected.key() == &row.key);
        let intent = intent.clone();
        let anchor = std::rc::Rc::clone(&control.anchor);
        controls = controls.child(
            kit::menu::trigger(
                control.id.clone(),
                &anchor,
                kit::menu::TriggerSizing::from_compact(touch),
                tokens,
            )
            .role(gpui::Role::Button)
            .aria_label(SharedString::from(Arc::clone(&row.menu_label)))
            .aria_expanded(expanded)
            .on_click(cx.listener(move |shell, _, window, cx| {
                cx.stop_propagation();
                shell.schedule_menu(&intent, &anchor, window, cx);
            })),
        );
    }
    controls.into_any_element()
}
