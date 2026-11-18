//! Paseo's combobox sheet header (TRACKER 798): the title alone.
//!
//! [Decision Log]
//! - Purpose: Paseo's tab switcher is a `Combobox` with a `title` and no
//!   `header`, so its sheet draws only the title under the handle
//!   (`combobox.tsx` `bottomSheetHeader`, `comboboxTitle`): no close button
//!   and no rule; Escape, the scrim and a pull on the handle close it.
//! - Reference values: 24 across (`spacing[6]`), 8 under (`spacing[2]`), the
//!   title at the base size, medium, in the foreground, left aligned.
//! - Choice: a header form of the kit dialog (`DialogInput::with_title_only`),
//!   so the sheet keeps the dialog's focus, Escape, scrim and layout.
//! - Cost: the form takes the title only; a back, actions, search or
//!   subtitle given with it are not drawn.

use gpui::prelude::*;
use gpui::{Div, SharedString, div, px};

use crate::fonts::Type;
use crate::popup::Surfaces;
use crate::theme::spacing::{S2, S6};
use crate::widgets::TypeExt as _;

/// The title row: the title under the handle, no close and no rule.
pub(super) fn row(title: SharedString, surfaces: &Surfaces) -> Div {
    div()
        .debug_selector(|| "dialog-header".to_owned())
        .flex()
        .flex_shrink_0()
        .px(px(S6))
        .pb(px(S2))
        .child(
            div()
                .debug_selector(|| "dialog-title".to_owned())
                .min_w(px(0.0))
                .type_(Type::Label)
                .text_color(surfaces.foreground)
                .child(title),
        )
}
