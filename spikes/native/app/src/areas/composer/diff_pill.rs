//! The composer's diff count (Paseo's `composer/diff-stat-pill.tsx`). It
//! shows when a side is greater than zero, and a press opens the session's
//! existing Changes tab. There is no second diff pane.

use gpui::ClickEvent;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, MouseButton,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::diff_count::format_diff_count;
use vessel_proto::registry::DiffStat;

use crate::shell::Shell;

/// The pill, when the open session's workspace has a line to show.
#[must_use]
pub(super) fn pill(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let (machine, session) = {
        let open = shell.areas.session.open.as_ref()?;
        (open.machine.clone(), open.session)
    };
    let project = shell
        .fleet
        .machines
        .iter()
        .find(|item| item.name == machine)?
        .sessions
        .iter()
        .find(|card| card.session == session)?
        .member_project();
    let workspace = shell
        .areas
        .labels
        .workspace_of(&machine, session, project)?;
    let stat = shell.diff_stat_of(&machine, workspace)?;
    if stat.additions == 0 && stat.deletions == 0 {
        return None;
    }
    let words = shell.t.lang.sidebar();
    Some(counts(stat, words.open_changes, tokens, cx))
}

fn counts(
    stat: DiffStat,
    label: &'static str,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    div()
        .id("composer-diff-pill")
        .self_start()
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .min_h(px(32.0))
        .px(px(12.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(tokens.border_subtle)
        .bg(tokens.btn_secondary_bg)
        .hover(|style| style.bg(tokens.btn_secondary_bg_hover))
        .cursor_pointer()
        .type_(Type::Meta)
        .role(gpui::Role::Button)
        .aria_label(label)
        .child(
            div()
                .text_color(tokens.st_ready)
                .child(format!("+{}", format_diff_count(stat.additions))),
        )
        .child(
            div()
                .text_color(tokens.danger)
                .child(format!("\u{2212}{}", format_diff_count(stat.deletions))),
        )
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(cx.listener(|shell, _: &ClickEvent, window, cx| {
            press(shell, window, cx);
        }))
        .into_any_element()
}

fn press(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    cx.stop_propagation();
    shell.layout_open_changes(window, cx);
}
