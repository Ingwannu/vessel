//! `ScheduleRow` source parts and edit activation; retained clicks carry the
//! exact admitted intent, never a later row-index/alias lookup.
use crate::shell::Shell;
use kit::ui::FluentBuilder as _;
use kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, div, px,
};
use std::sync::Arc;
use vessel_app_kit::{self as kit, Tokens, Type, TypeExt as _, theme::spacing};
use vessel_app_view::schedules::presentation::PreparedRow;

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
    let row_element = kit::settings::row_with_content(
        content.into_any_element(),
        Some(trailing),
        index == 0,
        touch,
        tokens,
    )
    .id(("schedules-row", index))
    .active(move |style| style.bg(tokens.neutral.surface3))
    .when(!touch, |row| {
        row.hover(|style| style.bg(tokens.neutral.surface2))
    });
    if let Some(intent) = row.intent.as_ref().filter(|intent| {
        intent.allows(
            &shell.areas.schedules.data.page,
            vessel_app_view::schedules::Action::Edit,
        )
    }) {
        let intent = intent.clone();
        row_element
            .cursor_pointer()
            .on_click(cx.listener(move |shell, _, window, cx| {
                super::editor::open_edit(shell, &intent, window, cx);
            }))
    } else {
        row_element
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
        shell.t.lang.schedules().status(row.status),
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
    if row.has_menu()
        && let Some(intent) = &row.intent
    {
        {
            let ui = &shell.areas.schedules.ui;
            if let Some(control) = ui.anchors.get(&row.key) {
                let intent = intent.clone();
                let anchor = std::rc::Rc::clone(&control.anchor);
                controls = controls.child(
                    kit::menu::trigger(
                        control.id.clone(),
                        &anchor,
                        kit::ControlSizing::from_compact(touch),
                        tokens,
                    )
                    .on_click(cx.listener(move |shell, _, window, cx| {
                        cx.stop_propagation();
                        shell.schedule_menu(&intent, &anchor, window, cx);
                    })),
                );
            }
        }
    }
    controls.into_any_element()
}
