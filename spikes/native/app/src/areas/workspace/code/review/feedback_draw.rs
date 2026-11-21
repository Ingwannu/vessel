//! The pieces "Send feedback" draws (`feedback.rs` decides; Paseo's
//! `review/feedback/surface.tsx`, `floating-action-button.tsx`): the
//! notice over the button, the phone's floating button, and the list's
//! room under the button.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    AnyElement, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint, glyph};
use vessel_app_view::workspace::review::feedback::Unavailable;

use crate::shell::Shell;

/// The notice's bottom: the button's 16 below, its 32, and 8 between
/// (Paseo's `controlHeightForLayout("md") + SPACING[4] + SPACING[2]`).
const NOTICE_BOTTOM: f32 = 56.0;
/// Paseo's floating action button (`floating-action-layout.ts`): 56 high,
/// its icon `ICON_SIZE.lg`.
const FAB: f32 = 56.0;
const FAB_ICON: f32 = 20.0;
/// The list's clearance under the button (`use-feedback.ts`): a phone's
/// button and its inset, or the md control and its 16, plus 44.
const CLEARANCE_PHONE: f32 = 116.0;
const CLEARANCE: f32 = 92.0;

/// The notice over the button: why it is off, or that the send failed.
pub(super) fn notice(
    shell: &Shell,
    unavailable: Option<Unavailable>,
    tokens: &Tokens,
) -> Option<Div> {
    let words = shell.words.review();
    let (text, colour) = if shell.review_feedback().failed {
        (words.feedback_failed(), tokens.neutral.destructive)
    } else {
        let text = match unavailable? {
            Unavailable::NoAgents => words.feedback_no_agents(),
            Unavailable::Disconnected => words.feedback_disconnected(),
            Unavailable::NoContext => words.feedback_no_context(),
        };
        (text, tokens.neutral.foreground)
    };
    Some(
        div()
            .absolute()
            .bottom(px(NOTICE_BOTTOM))
            .left(px(spacing::S4))
            .right(px(spacing::S4))
            .flex()
            .justify_center()
            .child(
                // Paseo's `accessibilityRole="alert"`: a screen reader says
                // why the button is off, or that the send failed, when it
                // appears.
                div()
                    .id("review-feedback-notice")
                    .role(gpui::Role::Alert)
                    .aria_label(vessel_app_kit::ui::SharedString::from(text.to_owned()))
                    .p(px(spacing::S2))
                    .rounded(px(radius::MD))
                    .bg(tokens.neutral.surface1)
                    .type_(Type::Meta)
                    .text_color(colour)
                    .child(text.to_owned()),
            ),
    )
}

/// The list's last row: room under the button while it shows (Paseo's
/// `contentInsetBottom`), nothing otherwise.
pub(in super::super) fn spacer(shell: &Shell, window: &Window) -> AnyElement {
    let shows = shell.review_key().is_some_and(|key| {
        !shell
            .areas
            .composer
            .drafts
            .reviews()
            .comments(&key)
            .is_empty()
    });
    let room = match (shows, breakpoint::phone(window)) {
        (false, _) => 0.0,
        (true, true) => CLEARANCE_PHONE,
        (true, false) => CLEARANCE,
    };
    div()
        .debug_selector(|| "review-room".to_owned())
        .w_full()
        .h(px(room))
        .into_any_element()
}

/// Paseo's `FloatingActionButton` on a phone: fully round, 56 high, the
/// send mark at its large size, then the words.
pub(super) fn fab(label: String, enabled: bool, tokens: &Tokens) -> Stateful<Div> {
    div()
        .id("review-feedback")
        // A press is the button's alone: on a phone it floats over the
        // session's composer, whose controls lie under it.
        .occlude()
        .h(px(FAB))
        .px(px(spacing::S4))
        .rounded_full()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        // Paseo's `floating-action-button.tsx`: `surface3`, `foreground`,
        // the md shadow, `interactionHighlight` while pressed.
        .bg(tokens.neutral.surface3)
        .text_color(tokens.neutral.foreground)
        .shadow(vessel_app_kit::popup::shadow_md(tokens))
        .when(enabled, |fab| {
            let pressed = tokens.neutral.interaction_highlight;
            fab.active(move |style| style.bg(pressed))
        })
        .type_(Type::Body)
        .role(gpui::Role::Button)
        .aria_label(vessel_app_kit::ui::SharedString::from(label.clone()))
        .when(enabled, vessel_app_kit::ui::Styled::cursor_pointer)
        .when(!enabled, |fab| fab.opacity(0.5))
        .child(glyph(Glyph::Send, FAB_ICON, tokens.neutral.foreground))
        .child(label)
}
