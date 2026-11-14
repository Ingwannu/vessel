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
//! - The same banner serves a live session whose released agent could
//!   not be resumed (protocol 12, ADR 0038): "The agent could not be
//!   resumed", the node's words, and Try again (`session.wake`); Paseo
//!   leaves such an agent closed and to be tried again. A released agent
//!   that has not failed has no banner: the next send resumes it, as in
//!   Paseo, and the bar says it is released.
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
use vessel_app_view::session::{
    MarkAction, ParkState, can_start_again, is_archived, is_live, park_state,
};
use vessel_proto::SessionMode;

use crate::shell::Shell;

/// What the banner's button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Again {
    /// Start an ended session again (`session.resume`).
    Start,
    /// Start a parked session's agent again after a failed resume
    /// (`session.wake`).
    Wake,
}

/// The banner over the composer card while the session shown has ended,
/// or while its released agent could not be resumed; `None` otherwise or
/// before the node describes it.
pub(super) fn banner(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.conversation;
    let info = area.info.as_deref()?;
    if !matches!(info.spec.mode, SessionMode::Structured { .. }) {
        return None;
    }
    let words = shell.t.lang.conversation();
    let neutral = tokens.neutral;
    let (heading, line, again) = if is_live(info) {
        // Live: only a released agent whose resume failed has a banner (a
        // released one resumes on the next send, as in Paseo).
        let Some(ParkState::ResumeFailed(why)) = park_state(info) else {
            return None;
        };
        (words.resume_failed_heading(), why, Some(Again::Wake))
    } else {
        let can = can_start_again(info);
        (
            words.ended_heading(is_archived(info)),
            words.ended_line(can).to_owned(),
            can.then_some(Again::Start),
        )
    };
    let narrow = breakpoint::phone(window);
    let state = if area.restarting {
        ButtonState::Disabled
    } else {
        ButtonState::Enabled
    };
    let button = again.map(|again| {
        let (id, label) = match again {
            Again::Start => (
                "ended-start-again",
                words.mark_action(MarkAction::StartAgain),
            ),
            Again::Wake => ("parked-resume", words.resume_again()),
        };
        button_with_kind(id, label, ButtonKind::Secondary, state, tokens)
            .flex_none()
            .when(state == ButtonState::Enabled, |button| {
                button.on_click(cx.listener(move |shell, _event, _window, cx| match again {
                    Again::Start => shell.conversation_start_again(cx),
                    Again::Wake => shell.conversation_wake(cx),
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
                .child(heading),
        )
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(line),
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
    /// Whether the session shown has ended and its agent cannot pick up
    /// where it left off (the banner says so and offers no Start again):
    /// the composer then sends nothing, since a send would only try to
    /// start it again. `false` while it runs, can be started again, or is
    /// not yet described.
    pub(crate) fn conversation_cannot_resume(&self) -> bool {
        self.areas
            .conversation
            .info
            .as_deref()
            .is_some_and(|info| !is_live(info) && !can_start_again(info))
    }

    /// Start the session shown again (the banner's button, the menu's
    /// row): once, until the node answers.
    pub(super) fn conversation_start_again(&mut self, cx: &mut Context<'_, Self>) {
        self.conversation_restart(Act::StartAgain, cx);
    }

    /// Resume the session shown's released agent after a failed resume
    /// (the banner's Try again): once, until the node answers.
    fn conversation_wake(&mut self, cx: &mut Context<'_, Self>) {
        self.conversation_restart(Act::Wake, cx);
    }

    /// Send `act` (start again, or wake) unless one is on its way.
    fn conversation_restart(&mut self, act: Act, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        if area.restarting {
            return;
        }
        area.restarting = true;
        area.again_refused = None;
        self.act(act);
        cx.notify();
    }
}
