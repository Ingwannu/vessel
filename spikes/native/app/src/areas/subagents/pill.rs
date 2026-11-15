//! Every active state keeps its own mark and count in a mixed fan-out.
use crate::shell::Shell;
use vessel_app_kit::theme::spacing::{S1, S2};
use vessel_app_kit::ui::{
    Context, Div, ElementId, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind, dot};
use vessel_app_view::subagents::PillKind;

pub(super) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let area = &shell.areas.subagents;
    let mut row = div().flex().items_center().flex_wrap().gap(px(S2));
    if area.segments.is_empty() {
        return row.child(
            button_with_kind(
                "subagents-open",
                area.pill.clone(),
                ButtonKind::Ghost,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(|shell, _, window, cx| shell.open_subagents(window, cx))),
        );
    }
    for (index, (kind, label)) in area.segments.iter().enumerate() {
        let color = match kind {
            PillKind::Failed => tokens.status().error.foreground,
            PillKind::Working => tokens.st_building,
        };
        row = row.child(
            div()
                .flex()
                .items_center()
                .gap(px(S1))
                .child(dot(color))
                .child(
                    button_with_kind(
                        ElementId::NamedInteger(
                            "subagents-status".into(),
                            super::draw::row_number(index),
                        ),
                        label.clone(),
                        ButtonKind::Ghost,
                        ButtonState::Enabled,
                        tokens,
                    )
                    .on_click(cx.listener(|shell, _, window, cx| shell.open_subagents(window, cx))),
                ),
        );
    }
    row
}
