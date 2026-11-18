//! The session's title bar: its title, its state when it is not running,
//! where it runs, and the Session actions (the page's `session-bar.tsx`
//! and `session-actions.tsx`; Paseo's header: 36 points tall with a
//! pointer, 44 on a phone, a hairline under it).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, MouseButton,
    MouseDownEvent, ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{MenuItem, Tokens, Type, TypeExt as _, breakpoint, popup};
use vessel_app_view::ConversationWords;
use vessel_app_view::session::{
    CONVERSATION_PROTOCOL, MarkAction, ParkState, can_rename, is_live, mark_actions, park_state,
    session_name,
};
use vessel_app_view::timeline::{EntryKind, Logs};
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
    // The node's answer to a rename names the bar at once; the title
    // the frame passes follows once the session's own report comes.
    let title = area
        .info
        .as_deref()
        .and_then(|info| session_name(info, area.timeline.title()))
        .unwrap_or(title);
    let renamable = area.info.as_deref().is_some_and(can_rename);
    let status = area
        .info
        .as_ref()
        .map(|info| &info.status)
        .filter(|status| !matches!(status, SessionStatus::Running));
    // A released agent reads as released (Paseo's word), not running.
    let parked = area
        .info
        .as_deref()
        .and_then(park_state)
        .map(|state| parked_chip(shell, &state, tokens, narrow));
    let badge = status
        .filter(|_| parked.is_none())
        .map(|status| status_badge(status, *words, tokens));
    let lost = lost_line(shell, tokens, narrow);
    let refused = area.refused.as_ref().map(|(_, why)| {
        div()
            .min_w(px(0.0))
            .truncate()
            .type_(Type::Meta)
            .text_color(tokens.neutral.destructive)
            .child(why.clone())
    });
    let actions = actions_button(shell, tokens, narrow, cx);
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .h(px(if narrow { 44.0 } else { 36.0 }))
        .px(px(12.0))
        .border_b_1()
        .border_color(surfaces.border_accent)
        .child(title_element(title, renamable, tokens, cx))
        .children(badge)
        .children(parked)
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_extra_muted)
                .child(detail),
        )
        // The workspace "…" right after the title group, as Paseo's header
        // (TRACKER 795, `layout::workspace_menu`).
        .child(crate::areas::layout::workspace_menu_trigger(
            shell,
            crate::areas::layout::WorkspaceMenuHeader::Bar,
            vessel_app_kit::ControlSizing::from_compact(narrow),
            tokens,
            cx,
        ))
        .child(div().flex_1())
        .children(lost)
        .children(refused)
        // Track F's slot: the code panel's toggle.
        .children(crate::areas::workspace::code_toggle(shell, tokens, narrow, cx))
        .child(actions)
        .into_any_element()
}

/// The session's state when it is not running, in its colour.
pub(super) fn status_badge(
    status: &SessionStatus,
    words: ConversationWords,
    tokens: &Tokens,
) -> Div {
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
        .bg(popup::Surfaces::of(tokens).surface2)
        .type_(Type::Meta)
        .text_color(color)
        .child(words.status_word(status))
}

/// The Session actions button: its menu opens where it was pressed. Named
/// "Session actions" so it is never taken for the workspace "…" beside the
/// title (TRACKER 795, until 797 moves these actions into the tab menu).
fn actions_button(
    shell: &Shell,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let surfaces = popup::Surfaces::of(tokens);
    div()
        .id("session-actions")
        .role(gpui::Role::Button)
        .aria_label(shell.words.layout().session_actions().to_owned())
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
        .child(glyph(Glyph::More, 16.0, tokens.neutral.foreground_muted))
}

/// The bar's word for a released agent, quiet as an idle session is, or
/// for a resume that failed, in the danger colour; its tooltip says what a
/// send does, or the node's words for the failure.
pub(super) fn parked_chip(
    shell: &Shell,
    state: &ParkState,
    tokens: &Tokens,
    narrow: bool,
) -> AnyElement {
    let area = &shell.areas.conversation;
    let words = shell.t.lang.conversation();
    let (word, color, tip) = match state {
        ParkState::Released => (
            words.parked_word(),
            tokens.neutral.foreground_extra_muted,
            words.parked_hint().to_owned(),
        ),
        ParkState::ResumeFailed(why) => (
            words.resume_failed_word(),
            tokens.neutral.destructive,
            why.clone(),
        ),
    };
    let chip = div()
        .id("session-parked")
        .flex_shrink_0()
        .px(px(6.0))
        .rounded(px(4.0))
        .bg(popup::Surfaces::of(tokens).surface2)
        .type_(Type::Meta)
        .text_color(color)
        .child(word);
    let (tips, place) = &area.parked_tip;
    div()
        .flex_shrink_0()
        .child(tips.on(chip, place, tip, narrow))
        .children(tips.render(tokens))
        .into_any_element()
}

/// The lost connection, once, in the app's words (the sidebar already
/// says the machine is unreachable); the transport's words only in the
/// line's tooltip (the coordinator's review, 2026-11-13: it was said three
/// times, twice in the transport's English).
pub(super) fn lost_line(shell: &Shell, tokens: &Tokens, narrow: bool) -> Option<AnyElement> {
    let area = &shell.areas.conversation;
    let why = area.lost.as_ref()?;
    let line = div()
        .id("session-lost")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .min_w(px(0.0))
        .child(vessel_app_kit::dot(tokens.danger))
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(shell.t.session_lost),
        );
    let (tips, place) = &area.lost_tip;
    Some(
        div()
            .min_w(px(0.0))
            .child(tips.on(line, place, why.clone(), narrow))
            .children(tips.render(tokens))
            .into_any_element(),
    )
}

/// The title: a button that opens the rename dialog while the session
/// lives, else text.
fn title_element(
    title: String,
    renamable: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let text = div()
        .min_w(px(0.0))
        .truncate()
        .type_(Type::Label)
        .text_color(tokens.neutral.foreground);
    #[cfg(test)]
    let text = text.debug_selector(|| "session-title-text".to_owned());
    if !renamable {
        return text.child(title).into_any_element();
    }
    let surface = tokens.neutral.surface2;
    let shown = title.clone();
    text.id("session-title")
        .px(px(6.0))
        .mx(px(-6.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(move |style| style.bg(surface))
        .on_click(cx.listener(move |shell, _event, window, cx| {
            shell.start_rename(&shown, window, cx);
        }))
        .child(title)
        .into_any_element()
}

/// The menu's rows, as their actions say.
pub(super) fn menu_items(shell: &Shell) -> Vec<MenuItem> {
    shell
        .areas
        .conversation
        .menu_rows
        .iter()
        .map(|action| session_menu_item(shell, *action))
        .collect()
}

/// One Session action's row: its words, its tone, and whether it waits for
/// a lifecycle change already under way. The bar's menu and the compact
/// tab switcher's row menu (TRACKER 789) draw the same rows.
pub(crate) fn session_menu_item(shell: &Shell, action: MenuAction) -> MenuItem {
    let words = shell.t.lang.conversation();
    let mut item = match action {
        MenuAction::Mark(MarkAction::Delete) => {
            MenuItem::danger(words.mark_action(MarkAction::Delete))
        }
        MenuAction::Mark(mark) => MenuItem::new(words.mark_action(mark)),
        MenuAction::CopyLink => MenuItem::new(shell.words.sessions().copy_link()),
        MenuAction::Close => MenuItem::new(shell.words.sessions().close()),
        MenuAction::CopyId => MenuItem::new(words.copy_session_id()).grouped(),
        MenuAction::Kill => MenuItem::danger(words.kill()),
        MenuAction::AgentLog => MenuItem::new(
            words.agent_log(
                Logs::all(shell.areas.conversation.timeline.entries())
                    .lines()
                    .count(),
            ),
        ),
    };
    if matches!(
        action,
        MenuAction::Close
            | MenuAction::Mark(
                MarkAction::Archive
                    | MarkAction::Unarchive
                    | MarkAction::Delete
                    | MarkAction::StartAgain
            )
    ) {
        item.disabled = shell.sessions_lifecycle_busy();
    }
    item
}

/// The Session actions this session offers, in the bar's order: its marks,
/// Copy session id, Copy link, Close once the node described it, Agent log
/// once the agent wrote a line, Kill while it lives.
pub(crate) fn session_menu_rows(shell: &Shell) -> Vec<MenuAction> {
    let area = &shell.areas.conversation;
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
    rows.push(MenuAction::CopyLink);
    if area.info.is_some() {
        rows.push(MenuAction::Close);
    }
    // The agent's own log, out of the conversation (slice M): here
    // once the agent has written a line.
    if area
        .timeline
        .entries()
        .iter()
        .any(|entry| matches!(entry.kind, EntryKind::Log { .. }))
    {
        rows.push(MenuAction::AgentLog);
    }
    if area.info.as_deref().is_none_or(is_live) {
        rows.push(MenuAction::Kill);
    }
    rows
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
        let rows = session_menu_rows(self);
        let area = &mut self.areas.conversation;
        area.menu_rows = rows;
        area.menu.open(at, window, cx);
        cx.notify();
    }
}
