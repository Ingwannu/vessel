//! Appearance input chrome over the existing registered single-line engine.
//! Reuses the shared inset-focus bridge; no selection, event subscription or
//! editing state is duplicated. Parsing and commit/restore remain in view.
use super::styles::project;
use super::{Kind, TextField, settings_frame};
use crate::{Tokens, Type, TypeExt as _, fonts};
use gpui::prelude::*;
use gpui::{AnyElement, App, SharedString, TextAlign, div, px};
use gpui_base::input::{Input, InputAccessibility};

const MIN_HEIGHT: f32 = 36.0;
const FAMILY_MAX: f32 = 280.0;
const NUMBER_WIDTH: f32 = 64.0;
const CONTENT_WIDTH: f32 = 80.0;
const INSET_X: f32 = 12.0;
const INSET_Y: f32 = 8.0;
const RADIUS: f32 = 6.0;
// Fits the source's minimum with its insets/border and the embedded face.
const LINE: f32 = 18.0;

/// Source profiles share paint while retaining their own width/alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceProfile {
    /// A family name: flexible up to the reference maximum, aligned left.
    Family,
    /// A size: fixed narrow field, aligned right.
    Number,
    /// A content maximum: fixed wider field, aligned right.
    Width,
}
impl TextField {
    /// Draw the source Appearance field. Select-all uses the owner's shared
    /// focus callback; Enter/blur/edit use the one existing engine subscription.
    #[must_use]
    pub fn appearance_element(
        &self,
        profile: AppearanceProfile,
        label: SharedString,
        tokens: &Tokens,
        cx: &mut App,
    ) -> AnyElement {
        let Kind::Line(state) = &self.kind else {
            return div().into_any_element();
        };
        let alignment = match profile {
            AppearanceProfile::Family => TextAlign::Left,
            AppearanceProfile::Number | AppearanceProfile::Width => TextAlign::Right,
        };
        state.update(cx, |state, cx| {
            state.set_text_align(alignment, cx);
            state.set_accessibility(InputAccessibility::new(None).with_label(Some(label)));
        });
        project(state, tokens, tokens.neutral.foreground_muted, cx);
        let frame = settings_frame(self)
            .flex()
            .items_center()
            .min_h(px(MIN_HEIGHT))
            .px(px(INSET_X))
            .py(px(INSET_Y))
            .rounded(px(RADIUS))
            .border_1()
            .border_color(tokens.neutral.border)
            .bg(tokens.neutral.surface2)
            .text_color(tokens.neutral.foreground)
            .type_(Type::Body)
            .line_height(px(fonts::ui(LINE)));
        let frame = match profile {
            AppearanceProfile::Family => frame.flex_1().min_w_0().max_w(px(FAMILY_MAX)),
            AppearanceProfile::Number => frame.w(px(NUMBER_WIDTH)).flex_shrink_0(),
            AppearanceProfile::Width => frame.w(px(CONTENT_WIDTH)).flex_shrink_0(),
        };
        frame
            .child(div().flex_1().min_w_0().child(Input::new(state)))
            .into_any_element()
    }
}
#[cfg(test)]
mod tests;
