//! An ended or archived session's banner over the composer card (the
//! page's `features/session/ended-banner.tsx`, in Paseo's archived-agent
//! callout, `components/archived-agent-callout.tsx`): what happened,
//! whether the agent can pick up where it left off, and Start again where
//! it can.
//!
//! [Decision Log]
//! - Purpose: an ended session showed its conversation with nothing over
//!   the composer saying it was archived, or that its agent could not go
//!   on; Start again was only in the Session actions menu.
//! - The references: the page puts "Session archived" or "Session ended",
//!   then "History is kept. ..." (two sentences, by whether the agent can
//!   resume), and its Start again, in a box over the composer. Paseo's
//!   callout for an archived agent is the native look: `surface1`, a
//!   hairline of `borderAccent`, radius 16, 12 by 16 inside on a phone and
//!   16 by 24 wider, 12 between, its words muted at 14, a secondary
//!   button that stops answering while its call goes, and the call's
//!   error under it in the danger colour.
//! - Choice: the page's words and button in Paseo's callout, first in the
//!   slot over the composer card (the page's place: before the docks).
//!   The button sends the menu's own act (`start again`); while it goes
//!   the button and the menu's row wait, and a refusal shows under the
//!   banner in the board's words for it (not in the bar, so it is said
//!   once). Once the node says the session runs, the session area follows
//!   it afresh (its log goes on after its final status), as a send to an
//!   ended session already does.
//! - Differences, kept: Paseo's callout centres one line; this one has the
//!   page's heading and line at the left and the button at the right.
//!   Paseo's button is plain words; the page's restore mark is not drawn
//!   because the kit's button takes words only. The phone padding follows
//!   the app's phone width (768), not Paseo's 600.
//! - Cost: a send typed into an ended session's composer still starts it
//!   again (the lead's composer); the banner says what the agent can do,
//!   and the composer's own line under the conversation says what sending
//!   does.

use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, breakpoint, button_with_kind,
};
use vessel_app_link::Act;
use vessel_app_view::session::{MarkAction, can_start_again, is_archived, is_live};
use vessel_proto::SessionMode;

use crate::shell::Shell;

/// The banner over the composer card while the session shown has ended;
/// `None` while it is live or not yet described.
pub(super) fn banner(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.conversation;
    let info = area.info.as_deref()?;
    if is_live(info) || !matches!(info.spec.mode, SessionMode::Structured { .. }) {
        return None;
    }
    let words = shell.t.lang.conversation();
    let neutral = tokens.neutral;
    let again = can_start_again(info);
    let narrow = breakpoint::phone(window);
    let state = if area.starting_again {
        ButtonState::Disabled
    } else {
        ButtonState::Enabled
    };
    let button = again.then(|| {
        button_with_kind(
            "ended-start-again",
            words.mark_action(MarkAction::StartAgain),
            ButtonKind::Secondary,
            state,
            tokens,
        )
        .flex_none()
        .when(state == ButtonState::Enabled, |button| {
            button.on_click(cx.listener(|shell, _event, _window, cx| {
                shell.conversation_start_again(cx);
            }))
        })
    });
    let text = div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .type_(Type::Label)
                .text_color(neutral.foreground)
                .child(words.ended_heading(is_archived(info))),
        )
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(words.ended_line(again)),
        );
    let callout = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .py(px(if narrow { 12.0 } else { 16.0 }))
        .px(px(if narrow { 16.0 } else { 24.0 }))
        .rounded(px(16.0))
        .border_1()
        .border_color(neutral.border_accent)
        .bg(neutral.surface1)
        .child(text)
        .children(button);
    let refused = area.again_refused.as_ref().map(|(code, message)| {
        div()
            .type_(Type::Body)
            .text_color(neutral.destructive)
            .text_center()
            .child(shell.t.lang.board().again_refused(*code, message))
    });
    Some(
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(callout)
            .children(refused)
            .into_any_element(),
    )
}

impl Shell {
    /// Start the session shown again (the banner's button, the menu's
    /// row): once, until the node answers.
    pub(super) fn conversation_start_again(&mut self, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        if area.starting_again {
            return;
        }
        area.starting_again = true;
        area.again_refused = None;
        self.act(Act::StartAgain);
        cx.notify();
    }
}
