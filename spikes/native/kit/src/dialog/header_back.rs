//! Paseo's sheet-header back arrow (`SheetHeader.back`, adaptive-modal-sheet.tsx
//! 409-424), drawn by the one dialog header before the title
//! (shared-dialog-header-search.md, "Header back").
//!
//! [Decision Log]
//! - Purpose: a sheet's inner view (the label manager's edit view) goes back
//!   from the header, not from a button in the body.
//! - Reference: a bare 18-point `ArrowLeft` in `foregroundMuted`, `foreground`
//!   while pressed, radius `lg` (8), no fill, no hover style, before the title
//!   in the same row; the close button stays.
//! - Alternatives: a header action (trailing, beside the close button: the
//!   wrong side); a body Ghost button (the returned look).
//! - Choice: a typed `HeaderBack` holding the owner's click handler; the dialog
//!   draws it. The pressed ink is a second, foreground glyph shown only while
//!   the group is pressed, as GPUI's pressed style needs a stateful element and
//!   the glyph is not one.
//! - Impact: Paseo's 8-point `hitSlop` is not reproduced (its web build has
//!   none either); the target is the glyph's 18-point box.

use crate::assets::Glyph;
use gpui::prelude::*;
use gpui::{App, ClickEvent, Div, Hsla, Stateful, Window, div, px, svg};
use std::rc::Rc;

/// The glyph's size (`ArrowLeft size={18}`).
pub(super) const SIZE: f32 = 18.0;
/// `headerBackButton.borderRadius` (`borderRadius.lg`).
const RADIUS: f32 = 8.0;
const GROUP: &str = "dialog-header-back";

type OnBack = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The owner's way back, drawn before the title.
pub struct HeaderBack {
    on_back: OnBack,
}

impl HeaderBack {
    /// `on_back` runs on a click; it must check its own current context.
    #[must_use]
    pub fn new(on_back: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        Self {
            on_back: Rc::new(on_back),
        }
    }
}

impl std::fmt::Debug for HeaderBack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderBack").finish_non_exhaustive()
    }
}

pub(super) fn button(back: HeaderBack, muted: Hsla, pressed: Hsla) -> Stateful<Div> {
    let on_back = back.on_back;
    div()
        .id(GROUP)
        .debug_selector(|| GROUP.to_owned())
        .group(GROUP)
        .relative()
        .flex_none()
        .size(px(SIZE))
        .rounded(px(RADIUS))
        .cursor_pointer()
        .on_click(move |event, window, cx| on_back(event, window, cx))
        .child(arrow(muted))
        .child(
            div()
                .id("dialog-header-back-pressed")
                .absolute()
                .inset_0()
                .opacity(0.0)
                .group_active(GROUP, |style| style.opacity(1.0))
                .child(arrow(pressed)),
        )
}

fn arrow(color: Hsla) -> gpui::Svg {
    svg()
        .path(Glyph::ArrowLeft.path())
        .size(px(SIZE))
        .text_color(color)
}
