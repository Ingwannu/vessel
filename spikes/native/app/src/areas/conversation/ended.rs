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
//! - Slice M (the owner on 0.0.0.0.13: sending as in Paseo): an ended
//!   session that can go on has no banner at all; the next message
//!   starts it again underneath and shows at once (`tail`). The banner is
//!   left for an archived session (Paseo's archived callout) and for one
//!   whose agent cannot go on. A released agent's failed resume is the
//!   header's chip; the next send retries, as Paseo's closed agent does.
//! - Cost: a send typed into an ended session's composer still starts it
//!   again (the lead's composer); the banner says what the agent can do,
//!   and the composer's own line under the conversation says what sending
//!   does.

use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _, Stateful,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, breakpoint, button_with_kind,
};
use vessel_app_link::{Act, ActKind};
use vessel_app_view::session::{MarkAction, can_start_again, is_archived, is_live};
use vessel_app_view::timeline::{EmptyFork, empty_fork};
use vessel_proto::{SessionInfo, SessionMode};

use crate::shell::Shell;

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
    let narrow = breakpoint::phone(window);
    // A fork that never held a conversation (F2): why, and Remove. Its
    // records say so before the node's description is read again.
    let status = area.timeline.status().unwrap_or(&info.status);
    if let Some(why) = empty_fork(status) {
        return Some(empty_fork_banner(shell, why, tokens, narrow, cx));
    }
    // Slice M (the owner, 2026-11-14: as in Paseo, the next message
    // starts an ended agent again): a session that ended and can go on has
    // no card; the person sends and the message shows at once. An archived
    // one keeps Paseo's archived callout, and one whose agent cannot go on
    // says so, since nothing sent there would be answered. A failed resume
    // of a released agent is the header's to say; the next send retries.
    if is_live(info) {
        return None;
    }
    if can_start_again(info) && !is_archived(info) {
        return None;
    }
    Some(ended_banner(shell, info, tokens, narrow, cx))
}

/// An empty fork's banner: why it holds nothing, and Remove, with a
/// refused Remove (the node's words) under it.
fn empty_fork_banner(
    shell: &Shell,
    why: EmptyFork,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.conversation;
    let words = shell.words.fork();
    let going = area.forking.removing();
    let remove = button_with_kind(
        "empty-fork-remove",
        words.remove(),
        ButtonKind::Secondary,
        if going {
            ButtonState::Disabled
        } else {
            ButtonState::Enabled
        },
        tokens,
    )
    .flex_none()
    .when(!going, |button| {
        button.on_click(cx.listener(|shell, _event, _window, cx| shell.empty_fork_remove(cx)))
    });
    let line = why.words(shell.words);
    let callout = callout((words.empty_heading(), line), Some(remove), tokens, narrow);
    let refused = area
        .refused
        .as_ref()
        .filter(|(kind, _)| *kind == ActKind::Delete)
        .map(|(_, message)| message.clone());
    with_refusal(callout, refused, tokens)
}

/// An ended session's banner: what happened and, where its agent can go
/// on, Start again, with a refused Start again under it in the board's
/// words.
fn ended_banner(
    shell: &Shell,
    info: &SessionInfo,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.conversation;
    let words = shell.t.lang.conversation();
    let (can, archived) = (can_start_again(info), is_archived(info));
    let state = if area.restarting || shell.sessions_lifecycle_busy() {
        ButtonState::Disabled
    } else {
        ButtonState::Enabled
    };
    let button = (can || archived).then(|| {
        button_with_kind(
            "ended-start-again",
            if archived {
                shell.words.sessions().unarchive()
            } else {
                words.mark_action(MarkAction::StartAgain)
            },
            ButtonKind::Secondary,
            state,
            tokens,
        )
        .flex_none()
        .when(state == ButtonState::Enabled, |button| {
            button.on_click(cx.listener(move |shell, _event, window, cx| {
                if archived {
                    let action = if can {
                        vessel_app_view::sessions::LifecycleAction::Resume
                    } else {
                        vessel_app_view::sessions::LifecycleAction::Unarchive
                    };
                    shell.prepare_session_lifecycle(action, window, cx);
                } else {
                    shell.conversation_start_again(cx);
                }
            }))
        })
    });
    let line = (
        words.ended_heading(archived),
        words.ended_line(can).to_owned(),
    );
    let callout = callout(line, button, tokens, narrow);
    let refused = area
        .again_refused
        .as_ref()
        .map(|(code, message)| shell.t.lang.board().again_refused(*code, message));
    with_refusal(callout, refused, tokens)
}

/// A callout with the node's refusal of its button's act under it, in the
/// danger colour.
fn with_refusal(callout: Div, refused: Option<String>, tokens: &Tokens) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(callout)
        .children(refused.map(|refused| {
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.destructive)
                .text_center()
                .child(refused)
        }))
        .into_any_element()
}

/// Paseo's callout: a heading and a line at the left, a button at the
/// right when there is one.
fn callout(
    (heading, line): (&'static str, impl Into<vessel_app_kit::ui::SharedString>),
    button: Option<Stateful<Div>>,
    tokens: &Tokens,
    narrow: bool,
) -> Div {
    let neutral = tokens.neutral;
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
                .child(line.into()),
        );
    div()
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
        .children(button)
}

impl Shell {
    /// Whether the session shown has ended and its agent cannot pick up
    /// where it left off (the banner says so and offers no Start again):
    /// the composer then sends nothing, since a send would only try to
    /// start it again. `false` while it runs, can be started again, or is
    /// not yet described.
    pub(crate) fn conversation_cannot_resume(&self) -> bool {
        let area = &self.areas.conversation;
        // An empty fork's records say so before its description does.
        area.empty_fork()
            || area.info.as_deref().map_or_else(
                || {
                    self.areas
                        .session
                        .open
                        .as_ref()
                        .is_some_and(|open| open.archived_at.is_some())
                },
                |info| is_archived(info) || (!is_live(info) && !can_start_again(info)),
            )
    }

    /// Start the session shown again (the banner's button, the menu's
    /// row): once, until the node answers.
    pub(super) fn conversation_start_again(&mut self, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        if area.restarting {
            return;
        }
        area.restarting = true;
        area.again_refused = None;
        self.act(Act::StartAgain);
        cx.notify();
    }
}
