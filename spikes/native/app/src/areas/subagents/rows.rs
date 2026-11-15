//! Virtualized managed/provider rows retain distinct identities and actions.
use super::Row;
use crate::shell::Shell;
use vessel_app_kit::theme::spacing::{S1, S2, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};

const VIEWPORT_SHARE: f32 = 0.45;
const NESTED_VIEWPORT_SHARE: f32 = 0.2;
pub(super) fn list(shell: &Shell, window: &Window, cx: &mut Context<'_, Shell>) -> Div {
    div()
        .h(px(f32::from(window.viewport_size().height)
            * if shell.areas.subagents.pane.is_some() {
                NESTED_VIEWPORT_SHARE
            } else {
                VIEWPORT_SHARE
            }))
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .child(shell.areas.subagents.list.element(cx.processor(
            |shell: &mut Shell, index: usize, _window, cx| {
                let tokens = vessel_app_kit::tokens(cx);
                row(shell, index, &tokens, cx)
            },
        )))
}
fn row(shell: &Shell, index: usize, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let area = &shell.areas.subagents;
    let Some(row) = area.rows.get(index) else {
        return div().into_any_element();
    };
    let parts = match row {
        Row::Managed(address) => managed_row(shell, index, address, tokens, cx),
        Row::Provider(id) => provider_row(shell, index, id, tokens, cx),
    };
    let Some(RowParts {
        label,
        status,
        open,
        extra,
    }) = parts
    else {
        return div().into_any_element();
    };
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(px(S1))
        .p(px(S2))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(S3))
                .child(open)
                .child(extra),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(format!("{label} · {status}")),
        )
        .into_any_element()
}
struct RowParts<'a> {
    label: &'a str,
    status: &'a str,
    open: AnyElement,
    extra: AnyElement,
}
fn managed_row<'a>(
    shell: &'a Shell,
    index: usize,
    address: &vessel_proto::SessionRef,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<RowParts<'a>> {
    let area = &shell.areas.subagents;
    let words = shell.words.subagents();
    let track = area.track.as_ref()?;

    let child = track
        .managed()
        .iter()
        .find(|row| row.target.address == *address)?;
    let card = vessel_app_view::Card::from_session(child.target.machine.as_str(), &child.info);
    let parent = track.target().clone();
    let detach_parent = parent.clone();
    let address = *address;
    let open = button_with_kind(
        ElementId::NamedInteger("subagent-open".into(), super::draw::row_number(index)),
        card.title,
        ButtonKind::Ghost,
        ButtonState::Enabled,
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| {
        if shell
            .areas
            .subagents
            .track
            .as_ref()
            .is_some_and(|track| track.target() == &parent)
        {
            shell.subagent_managed(address, window, cx);
        }
    }))
    .into_any_element();
    let detach = button_with_kind(
        ElementId::NamedInteger("subagent-detach".into(), super::draw::row_number(index)),
        words.detach(),
        ButtonKind::Ghost,
        if area.detaching.is_none() && area.uncertain.is_none() {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| {
        if shell
            .areas
            .subagents
            .track
            .as_ref()
            .is_some_and(|track| track.target() == &detach_parent)
        {
            shell.subagent_confirm(address, window, cx);
        }
    }))
    .into_any_element();
    Some(RowParts {
        label: words.managed(),
        status: managed_status(&child.info, shell),
        open,
        extra: detach,
    })
}
fn provider_row<'a>(
    shell: &'a Shell,
    index: usize,
    id: &vessel_proto::subagents::ProviderChildId,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<RowParts<'a>> {
    let area = &shell.areas.subagents;
    let words = shell.words.subagents();
    let track = area.track.as_ref()?;
    let parent = track.target().clone();
    let hidden_parent = parent.clone();

    let scope = area.pane.as_ref().map(|pane| pane.query().child.clone());
    let child = track.providers(scope.as_ref()).find(|row| &row.id == id)?;
    let hide_scope = scope.clone();
    let title = child
        .description
        .as_ref()
        .filter(|text| !text.as_str().trim().is_empty())
        .or(child.title.as_ref())
        .map_or(
            words.untitled(),
            vessel_proto::subagents::ProviderText::as_str,
        );
    let chosen = id.clone();
    let hidden = id.clone();
    let open = button_with_kind(
        ElementId::NamedInteger("provider-open".into(), super::draw::row_number(index)),
        title.to_owned(),
        ButtonKind::Ghost,
        ButtonState::Enabled,
        tokens,
    )
    .on_click(cx.listener(move |shell, _, _, cx| {
        if shell
            .areas
            .subagents
            .pane
            .as_ref()
            .map(|pane| &pane.query().child)
            == scope.as_ref()
            && shell
                .areas
                .subagents
                .track
                .as_ref()
                .is_some_and(|track| track.target() == &parent)
        {
            shell.subagent_provider(&chosen);
            cx.notify();
        }
    }))
    .into_any_element();
    let hide = button_with_kind(
        ElementId::NamedInteger("provider-hide".into(), super::draw::row_number(index)),
        words.hide(),
        ButtonKind::Ghost,
        ButtonState::Enabled,
        tokens,
    )
    .on_click(cx.listener(move |shell, _, _, cx| {
        if shell
            .areas
            .subagents
            .pane
            .as_ref()
            .map(|pane| &pane.query().child)
            == hide_scope.as_ref()
            && shell
                .areas
                .subagents
                .track
                .as_ref()
                .is_some_and(|track| track.target() == &hidden_parent)
        {
            shell.subagent_hide(&hidden);
            cx.notify();
        }
    }))
    .into_any_element();
    Some(RowParts {
        label: words.provider(),
        status: provider_status(child.status, shell),
        open,
        extra: hide,
    })
}
pub(super) fn provider_status(
    status: Option<vessel_proto::subagents::ChildStatus>,
    shell: &Shell,
) -> &str {
    use vessel_proto::subagents::ChildStatus;
    let words = shell.words.subagents();
    match status {
        Some(ChildStatus::Running) => words.running(),
        Some(ChildStatus::Completed) => words.completed(),
        Some(ChildStatus::Failed) => words.failed(),
        Some(ChildStatus::Canceled) => words.canceled(),
        None => words.unknown(),
    }
}
fn managed_status<'a>(info: &vessel_proto::SessionInfo, shell: &'a Shell) -> &'a str {
    use vessel_proto::{Activity, SessionStatus};
    let words = shell.words.subagents();
    if matches!(info.status, SessionStatus::Failed { .. }) {
        return words.failed();
    }
    match info.board.as_ref().map(|board| board.activity) {
        Some(Activity::Active) => words.running(),
        Some(Activity::Idle) => words.idle(),
        Some(Activity::WaitingInput) => words.waiting(),
        Some(Activity::Exited) => words.ended(),
        None => words.unknown(),
    }
}
