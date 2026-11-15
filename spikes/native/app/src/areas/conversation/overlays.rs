//! What floats over the conversation's frame: the Session actions menu
//! when open, the confirm when one is asked, the agent's log, and the
//! menus and tooltips of going back (rewind, fork).

use vessel_app_kit::output::Lines;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{
    DialogContent, DialogSize, Start, Tokens, Type, TypeExt as _, breakpoint, button, dialog,
    lines_box, menu,
};
use vessel_app_view::timeline::Logs;

use super::Confirm;
use crate::shell::Shell;

/// The agent log's box: at most this tall, then its lines scroll.
const LOG_HEIGHT: f32 = 420.0;

/// The menus and tooltips of going back in the conversation: rewind
/// under a person's message (`rewind`), fork under a turn's footer
/// (`fork`).
fn going_back(
    shell: &Shell,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let area = &shell.areas.conversation;
    let mut floating = Vec::new();
    floating.extend(menu(
        "rewind-menu",
        &area.rewinding.menu,
        &super::rewind::menu_items(shell),
        &Shell::rewind_menu_handlers(),
        tokens,
        narrow,
        cx,
    ));
    floating.extend(area.rewinding.tips.render(tokens));
    floating.extend(menu(
        "fork-menu",
        &area.forking.menu,
        &super::fork::menu_items(shell),
        &Shell::fork_menu_handlers(),
        tokens,
        narrow,
        cx,
    ));
    floating.extend(area.forking.tips.render(tokens));
    floating
}

/// What floats over the frame: the Session actions menu when open, the
/// confirm when one is asked.
pub(crate) fn overlays(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let area = &shell.areas.conversation;
    let narrow = breakpoint::phone(window);
    let mut floating = Vec::new();
    floating.extend(super::search::example(shell, tokens, window, cx));
    floating.extend(going_back(shell, tokens, narrow, cx));
    let items = super::bar::menu_items(shell);
    if let Some(menu) = menu(
        "session-actions",
        &area.menu,
        &items,
        &Shell::menu_handlers(),
        tokens,
        narrow,
        cx,
    ) {
        floating.push(menu);
    }
    if let Some(confirm) = area.confirm {
        floating.push(confirm_dialog(shell, confirm, tokens, (window, narrow), cx));
    }
    if area.agent_log == super::AgentLog::Shown {
        floating.push(agent_log(shell, tokens, window, narrow, cx));
    }
    floating
}

/// The confirm asked of the person: archive, delete or kill, its words
/// naming the machine, a destructive accept for delete and kill.
fn confirm_dialog(
    shell: &Shell,
    confirm: Confirm,
    tokens: &Tokens,
    (window, narrow): (&mut Window, bool),
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.conversation;
    let words = shell.t.lang.conversation();
    let machine = area
        .shown
        .as_ref()
        .map(|shown| shown.machine.clone())
        .unwrap_or_default();
    let short: String = area
        .shown
        .as_ref()
        .map(|shown| shown.session.to_string().chars().take(4).collect())
        .unwrap_or_default();
    let (title, body, accept) = match confirm {
        Confirm::Kill => (
            words.kill_title(&short),
            words.kill_words(&machine),
            words.kill(),
        ),
    };
    let dismiss = words.cancel();
    let footer = vec![
        button("confirm-dismiss", dismiss, false, tokens)
            .on_click(cx.listener(|shell, _event, window, cx| shell.dismiss_confirm(window, cx)))
            .into_any_element(),
        button("confirm-accept", accept, true, tokens)
            .bg(tokens.neutral.destructive)
            .on_click(cx.listener(|shell, _event, _window, cx| shell.confirmed(cx)))
            .into_any_element(),
    ];
    dialog(
        &area.dialog,
        DialogContent {
            title: title.into(),
            body: div().type_(Type::Body).child(body).into_any_element(),
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::dismiss_confirm,
        cx,
    )
}

/// The agent's own log (its adapter's lines on stderr), the whole of it
/// in one box: the Session actions' "Agent log". Its lines were rows
/// between turns; Paseo shows no adapter log in the stream (slice M).
fn agent_log(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.conversation;
    let words = shell.t.lang.conversation();
    let lines = Logs::all(area.timeline.entries());
    let count = lines.lines().count();
    let body = div()
        .max_h(px(LOG_HEIGHT))
        .child(lines_box(
            "agent-log-lines",
            u64::try_from(count).unwrap_or(u64::MAX),
            || Lines::text(&lines),
            Start::End,
            tokens,
            window,
            cx,
        ))
        .into_any_element();
    let footer = vec![
        button("agent-log-close", words.close(), false, tokens)
            .on_click(cx.listener(|shell, _event, window, cx| shell.dismiss_log(window, cx)))
            .into_any_element(),
    ];
    dialog(
        &area.dialog,
        DialogContent {
            title: words.agent_log(count).into(),
            body,
            footer,
            size: DialogSize::Wide,
        },
        tokens,
        window,
        narrow,
        Shell::dismiss_log,
        cx,
    )
}
