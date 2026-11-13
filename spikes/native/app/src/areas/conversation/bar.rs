//! The session's title bar: its title, its state when it is not running,
//! where it runs, and the Session actions (the page's `session-bar.tsx`
//! and `session-actions.tsx`; Paseo's header: 36 points tall with a
//! pointer, 44 on a phone, a hairline under it).

use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, MouseButton, MouseDownEvent,
    ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{MenuItem, Tokens, Type, TypeExt as _, breakpoint, popup};
use vessel_app_view::session::{CONVERSATION_PROTOCOL, MarkAction, is_live, mark_actions};
use vessel_proto::SessionStatus;

use super::MenuAction;
use crate::shell::Shell;

/// The bar: `title` (the session's name), `detail` (where it runs).
pub(crate) fn bar(
    shell: &Shell,
    title: String,
    detail: String,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.conversation;
    let words = shell.t.lang.conversation();
    let surfaces = popup::Surfaces::of(tokens);
    let narrow = breakpoint::phone(window);
    let status = area
        .info
        .as_ref()
        .map(|info| &info.status)
        .filter(|status| !matches!(status, SessionStatus::Running));
    let badge = status.map(|status| {
        let color = match status {
            SessionStatus::Starting => tokens.st_building,
            SessionStatus::Running => tokens.st_ready,
            SessionStatus::Failed { .. } => tokens.neutral.destructive,
            SessionStatus::Exited { .. } | SessionStatus::Interrupted => {
                tokens.neutral.foreground_extra_muted
            }
        };
        div()
            .flex_shrink_0()
            .px(px(6.0))
            .rounded(px(4.0))
            .bg(surfaces.surface2)
            .type_(Type::Meta)
            .text_color(color)
            .child(words.status_word(status))
    });
    let refused = area.refused.as_ref().map(|(_, why)| {
        div()
            .min_w(px(0.0))
            .truncate()
            .type_(Type::Meta)
            .text_color(tokens.neutral.destructive)
            .child(why.clone())
    });
    let actions = div()
        .id("session-actions")
        .flex_shrink_0()
        .size(px(if narrow { 44.0 } else { 28.0 }))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(popup::MENU_RADIUS))
        .text_color(tokens.neutral.foreground_muted)
        .cursor_pointer()
        .hover(move |style| style.bg(surfaces.surface2))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|shell, event: &MouseDownEvent, window, cx| {
                shell.open_actions(event.position, window, cx);
                cx.stop_propagation();
            }),
        )
        .child("⋯");
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .h(px(if narrow { 44.0 } else { 36.0 }))
        .px(px(12.0))
        .border_b_1()
        .border_color(surfaces.border_accent)
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .type_(Type::Label)
                .text_color(tokens.neutral.foreground)
                .child(title),
        )
        .children(badge)
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_extra_muted)
                .child(detail),
        )
        .child(div().flex_1())
        .children(refused)
        .child(actions)
        .into_any_element()
}

/// The menu's rows, as their actions say.
pub(super) fn menu_items(shell: &Shell) -> Vec<MenuItem> {
    let words = shell.t.lang.conversation();
    shell
        .areas
        .conversation
        .menu_rows
        .iter()
        .map(|action| match action {
            MenuAction::Mark(MarkAction::Delete) => {
                MenuItem::danger(words.mark_action(MarkAction::Delete))
            }
            MenuAction::Mark(mark) => MenuItem::new(words.mark_action(*mark)),
            MenuAction::CopyId => MenuItem::new(words.copy_session_id()).grouped(),
            MenuAction::Kill => MenuItem::danger(words.kill()),
        })
        .collect()
}

impl Shell {
    /// Open the Session actions at `at`, with the rows this session
    /// offers.
    fn open_actions(
        &mut self,
        at: vessel_app_kit::ui::Point<vessel_app_kit::ui::Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.conversation;
        // The fleet does not say a machine's protocol yet; a session the
        // node described is from one with marks (protocol 10), and an
        // older node refuses a mark it does not know, which is shown.
        let mut rows: Vec<MenuAction> = area
            .info
            .as_deref()
            .map(|info| {
                mark_actions(info, Some(CONVERSATION_PROTOCOL))
                    .into_iter()
                    .map(MenuAction::Mark)
                    .collect()
            })
            .unwrap_or_default();
        rows.push(MenuAction::CopyId);
        if area.info.as_deref().is_none_or(is_live) {
            rows.push(MenuAction::Kill);
        }
        area.menu_rows = rows;
        area.menu.open(at, window, cx);
        cx.notify();
    }
}
