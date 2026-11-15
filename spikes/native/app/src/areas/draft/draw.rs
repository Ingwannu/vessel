//! The existing draft renderer, extracted solely for the shared file budget.
use super::{Shown, effort_card, pickers, status};
use crate::{
    areas::composer::{attach_button, draft_card},
    shell::Shell,
};
use vessel_app_kit::ui::{
    Context, Div, Hsla, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{SendKind, Tokens, Type, TypeExt as _, send_button};
use vessel_app_view::{Machine, agent_name};

/// Whether the window is at Paseo's compact width (touch-sized controls,
/// pickers as sheets).
pub(crate) fn compact(window: &Window) -> bool {
    window.viewport_size().width < px(vessel_app_kit::picker::COMPACT)
}

/// The draft screen.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    shell.creation_editor(cx);
    let restored = shell.areas.draft.restore.borrow_mut().take();
    if let Some(text) = restored {
        // A failed start's text, back in the composer (once).
        shell.areas.draft.composer.set_text(&text, window, cx);
    }
    let t = shell.t;
    let draft = &shell.areas.draft;
    let online: Vec<&Machine> = shell.fleet.online().collect();
    let shown = shell.shown();
    let touch = compact(window);
    let neutral = tokens.neutral;

    let typed = !draft.composer.text(cx).trim().is_empty() || shell.draft_has_files();
    let can_send = typed
        && draft.agent.is_some()
        && draft.creation.flow.may_submit()
        && !shell.composer_attachments_blocked(vessel_app_view::composer::drafts::DraftKey::New);
    let attach = attach_button(draft.creation.flow.may_submit(), touch, tokens, cx);
    let card = draft_card(shell, tokens, touch, window, cx).child(
        // Paseo's button row bleeds 6 past the card's padding.
        div()
            .mx(px(-6.0))
            .flex()
            .flex_row()
            .items_end()
            .justify_between()
            .gap(px(8.0))
            .child(attach)
            .child(pickers::controls(shell, shown.as_ref(), touch, tokens, window, cx).flex_1())
            .child(
                send_button("send-draft", SendKind::Send, can_send, touch, tokens).on_click(
                    cx.listener(|this, _event, window, cx| {
                        let text = this.areas.draft.composer.text(cx);
                        this.send_draft(text, window, cx);
                    }),
                ),
            ),
    );

    let mut column = div()
        .w_full()
        .max_w(px(820.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        // The brand's drawing measures the heading, the card and the
        // pickers (worker B's `brand.rs`); it is the mark, drawn large.
        .on_children_prepainted(shell.brand.new_session(0))
        .child(
            div()
                // As wide as its words, so the brand's drawing measures the
                // text and not the column (worker B's `brand.rs`).
                .self_start()
                .type_(Type::Title)
                .text_size(px(vessel_app_kit::fonts::ui(22.0)))
                .line_height(px(vessel_app_kit::fonts::ui(28.0)))
                .text_color(neutral.foreground)
                .child(t.welcome),
        )
        .child(card)
        .child(pickers::place(shell, &online, touch, tokens, cx).self_start());
    if let Some(error) = &draft.error {
        column = column.child(note(error.clone(), neutral.destructive));
    }
    if let Some(status) = status::render(shell, tokens, cx) {
        column = column.child(status);
    }
    if let (Some(Shown::Failed(why)), Some(agent)) = (&shown, &draft.agent) {
        let words = t.lang.options();
        let failed = words.failed(&format!("{} — {why}", agent_name(agent)));
        column = column.child(note(failed, neutral.foreground_muted));
    }
    if shell.loaded && online.is_empty() {
        column = column.child(note(
            if shell.fleet.machines.is_empty() {
                t.no_machine_yet
            } else {
                t.no_machine_connected
            }
            .to_owned(),
            neutral.foreground_muted,
        ));
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .px(px(if touch { 12.0 } else { 32.0 }))
        .child(column)
        .children(overlays(
            shell,
            &online,
            shown.as_ref(),
            tokens,
            touch,
            window,
            cx,
        ))
}

/// A running session's controls: the draft's pills on its options (the
/// session target, `Shell::controls_target`) and what is drawn over them
/// (the effort card, the tooltip, the open list); `None` until the
/// session's agent reported options.
pub(crate) fn running_controls(
    shell: &Shell,
    touch: bool,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<(Div, Vec<vessel_app_kit::ui::AnyElement>)> {
    let shown = shell.shown()?;
    let row = pickers::controls(shell, Some(&shown), touch, tokens, window, cx);
    let online: Vec<&Machine> = shell.fleet.online().collect();
    let over = overlays(shell, &online, Some(&shown), tokens, touch, window, cx);
    Some((row, over))
}

/// What is drawn over the new chat: the features sheet, the composer's
/// tooltip, and the open list.
fn overlays(
    shell: &Shell,
    online: &[&Machine],
    shown: Option<&Shown>,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<vessel_app_kit::ui::AnyElement> {
    let ready = match shown {
        Some(Shown::Ready(controls)) => Some(controls.as_ref()),
        Some(Shown::Loading | Shown::Failed(_)) | None => None,
    };
    let mut over = Vec::new();
    over.extend(effort_card::card(shell, ready, touch, tokens, window, cx));
    over.extend(shell.areas.draft.features.tips.render(tokens));
    over.extend(pickers::open_list(shell, online, shown, tokens, window, cx));
    over
}

/// A sentence under the composer.
fn note(text: String, color: Hsla) -> Div {
    div().type_(Type::Meta).text_color(color).child(text)
}
