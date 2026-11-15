//! The pull request at the head of the Changes tab: Paseo's
//! `ChangesPullRequestLink` (`git/diff-pane.tsx`): its state's glyph and
//! "#42" as a toolbar label, "Pull request" on hover; pressing it opens
//! the facts. Paseo's external-link button beside it is the facts'
//! "View" here (one way to the page, not two side by side on a phone's
//! narrow row).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint};
use vessel_app_view::workspace::pull::{PullEntry, PullPhase};
use vessel_proto::workspace::GitStatus;

use crate::shell::Shell;

/// The kit toolbar label's corner (`toolbar_label::RADIUS`).
const RADIUS: f32 = 6.0;
/// Paseo's `PullRequestStateIcon` at 14.
const STATE_GLYPH: f32 = 14.0;

/// Paseo's state presentation (`PR_STATE_PRESENTATION`): open in the
/// success colour, a draft muted, merged in the accent (Paseo's purple is
/// not in this palette), closed in the error colour.
pub(super) fn state_glyph(phase: PullPhase, tokens: &Tokens) -> AnyElement {
    state_glyph_sized(phase, STATE_GLYPH, tokens)
}

/// [`state_glyph`] at `size` (a card's meta row draws it at 12).
pub(super) fn state_glyph_sized(phase: PullPhase, size: f32, tokens: &Tokens) -> AnyElement {
    let status = tokens.status();
    let (which, colour) = match phase {
        PullPhase::Open => (Glyph::PullRequest, status.success.foreground),
        PullPhase::Draft => (Glyph::PullRequestDraft, tokens.neutral.foreground_muted),
        PullPhase::Merged => (Glyph::GitMerge, tokens.neutral.accent),
        PullPhase::Closed => (Glyph::PullRequestClosed, status.error.foreground),
        PullPhase::Unknown => (Glyph::PullRequest, tokens.neutral.foreground_muted),
    };
    glyph(which, size, colour).flex_none().into_any_element()
}

/// The link, when the session's pull request has been read.
pub(crate) fn link(
    shell: &Shell,
    status: &GitStatus,
    tokens: &Tokens,
    window: &vessel_app_kit::ui::Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let PullEntry::View { number, phase, .. } = shell.pulls_entry(status) else {
        return None;
    };
    let pulls = &shell.areas.pulls;
    let touch = breakpoint::phone(window);
    let neutral = tokens.neutral;
    let open = pulls.facts.is_some();
    let element = div()
        .id("pulls-link")
        .flex()
        .flex_none()
        .items_center()
        .gap(px(spacing::S1))
        .px(px(spacing::S1))
        .py(px(spacing::S1))
        .rounded(px(RADIUS))
        .type_(Type::Meta)
        .text_color(if open {
            neutral.foreground
        } else {
            neutral.foreground_muted
        })
        .cursor_pointer()
        .hover(move |style| {
            style
                .bg(neutral.interaction_highlight)
                .text_color(neutral.foreground)
        })
        .when(touch, |frame| frame.min_h(px(breakpoint::TOUCH_TARGET)))
        .child(state_glyph(phase, tokens))
        .child(format!("#{}", number.get()))
        .on_click(cx.listener(|shell, _event, window, cx| shell.pulls_open_facts(window, cx)));
    Some(
        pulls
            .tips
            .on(element, &pulls.link_tip, shell.words.pulls().label(), touch)
            .into_any_element(),
    )
}
