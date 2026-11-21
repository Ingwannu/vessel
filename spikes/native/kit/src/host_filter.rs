//! Paseo's `HostFilter` trigger (`components/hosts/host-filter.tsx`): the
//! pill that names the host shown and opens the host list. The label
//! manager (`manager-modal.tsx` 188-197) and the plugin screen header
//! (`plugins/surface-screen.tsx` 177-189) show it when there is more than
//! one host, without "All hosts".
//!
//! [Decision Log]
//! - Purpose: one trigger for every screen that picks a host, as Paseo
//!   shares `HostFilter`.
//! - History: the label manager drew it first (`app/areas/labels/
//!   host_filter.rs`, the coordinator's RETURN on the manager body,
//!   2026-11-16). Its log said the trigger would move here unchanged when a
//!   second screen needed it; the plugin screen header is that screen
//!   (coordinator's RETURN on P3b-1 native N1, 2026-11-21).
//! - Reference: the host's status dot (8 in a 14 slot), its name in 14
//!   medium `foreground` and a 14 muted chevron, gap 6, padding 6 by 12,
//!   radius 6, `surface1` with a `border` hairline, `surface2` on hover and
//!   `surface3` while pressed, sized to its content.
//! - Choice: the kit draws the trigger and its pressed states; the owner
//!   adds the press and keeps the list (the kit's combobox under the
//!   trigger, `bottom-start`, one row per host with its status dot and a
//!   check on the one shown, no search).

use gpui::prelude::*;
use gpui::{Div, Hsla, SharedString, Stateful, div, px};

use crate::assets::Glyph;
use crate::theme::radius::MD;
use crate::theme::spacing::{S1_5, S3};
use crate::{Tokens, Type, TypeExt as _, anchor_probe, dot, glyph, select};

/// `HostStatusDot`: 8 across, in an `iconSize.sm` slot.
const DOT: f32 = 8.0;
const DOT_SLOT: f32 = 14.0;
/// The chevron's size.
const CARET: f32 = 14.0;

/// The trigger naming `label`, whose host is `online`; where it is drawn
/// goes to `anchor`. Enabled, it takes the pointer and the pressed states;
/// the owner adds what a press does.
#[must_use]
pub fn trigger(
    id: &'static str,
    label: impl Into<SharedString>,
    online: bool,
    enabled: bool,
    anchor: &select::Anchor,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    div()
        .id(id)
        .debug_selector(move || id.to_owned())
        .relative()
        .flex()
        .flex_row()
        .flex_none()
        .self_start()
        .items_center()
        .gap(px(S1_5))
        .py(px(S1_5))
        .px(px(S3))
        .rounded(px(MD))
        .bg(neutral.surface1)
        .border_1()
        .border_color(neutral.border)
        .child(anchor_probe(anchor))
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(DOT_SLOT))
                .child(dot(status(online, tokens)).size(px(DOT))),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .type_(Type::Label)
                .text_color(neutral.foreground)
                .child(label.into()),
        )
        .child(glyph(Glyph::ChevronDown, CARET, neutral.foreground_muted))
        .when(enabled, |pill| {
            pill.cursor_pointer()
                .hover(move |style| style.bg(neutral.surface2))
                .active(move |style| style.bg(neutral.surface3))
        })
}

/// Paseo's host status dot (`HostStatusDot`): the connection's state, not
/// a label contract's: `statusSuccess` while connected (an Unsupported host
/// too), `statusDanger` otherwise.
#[must_use]
pub fn status(online: bool, tokens: &Tokens) -> Hsla {
    if online {
        tokens.st_ready
    } else {
        tokens.danger
    }
}
