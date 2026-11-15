//! Paseo's labeled form frame; content is already localized by its caller.
//!
//! [Decision Log]
//! - Purpose: one label/trailing/body/feedback layout for forms in every area.
//! - Existing assets: `TextField` and select own input chrome/engines; neither
//!   owns this surrounding Field layout. Settings rows have different geometry.
//! - Choice: an additive content renderer, with no parsing or domain state.
//! - Source: pinned ui/form-field.tsx37–78/238–264: label14 normal/muted,
//!   gap8, subtext12/17, error red300 before hint, one error line.
//! - Why/cost: screens cannot drift by reproducing this frame independently;
//!   caller still owns validation, localized words, focus and field lifetime.
use crate::{Tokens, Type, TypeExt as _, fonts, theme::spacing};
use gpui::prelude::*;
use gpui::{AnyElement, Div, Hsla, SharedString, div, px};

const SUBTEXT_LINE_HEIGHT: f32 = 17.0;
/// Localized content around an existing input or selector. No field is created.
pub struct Content {
    /// Field label; may wrap when a trailing control needs space.
    pub label: SharedString,
    /// Optional caller-owned header action/extra text, after the label.
    pub trailing: Option<AnyElement>,
    /// The existing field/selector, unchanged by this frame.
    pub body: AnyElement,
    /// Helpful text when there is no current error.
    pub hint: Option<SharedString>,
    /// Current error takes precedence over hint, as the source Field does.
    pub error: Option<SharedString>,
}
impl std::fmt::Debug for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Content")
            .field("label", &self.label)
            .field("hint", &self.hint)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeedbackKind {
    Hint,
    Error,
}
fn feedback<'a>(
    hint: Option<&'a SharedString>,
    error: Option<&'a SharedString>,
) -> Option<(FeedbackKind, &'a SharedString)> {
    if let Some(error) = error.filter(|text| !text.is_empty()) {
        Some((FeedbackKind::Error, error))
    } else {
        hint.filter(|text| !text.is_empty())
            .map(|hint| (FeedbackKind::Hint, hint))
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Geometry {
    gap: f32,
    subtext_line: f32,
}
fn geometry() -> Geometry {
    Geometry {
        gap: spacing::S2,
        subtext_line: fonts::ui(SUBTEXT_LINE_HEIGHT),
    }
}
fn ink(kind: FeedbackKind, tokens: &Tokens) -> Hsla {
    match kind {
        FeedbackKind::Hint => tokens.neutral.foreground_muted,
        FeedbackKind::Error => tokens.status().red_300,
    }
}
/// The source Field frame, around one caller-owned existing control.
#[must_use]
pub fn frame(content: Content, tokens: &Tokens) -> Div {
    let geometry = geometry();
    let mut header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(geometry.gap))
        .child(
            div()
                .flex_shrink_1()
                .min_w_0()
                .type_(Type::Body)
                .font_weight(gpui::FontWeight::NORMAL)
                .text_color(tokens.neutral.foreground_muted)
                .whitespace_normal()
                .child(content.label),
        );
    if let Some(trailing) = content.trailing {
        header = header.child(trailing);
    }
    let line = feedback(content.hint.as_ref(), content.error.as_ref()).map(|(kind, text)| {
        div()
            .min_w_0()
            .type_(Type::Meta)
            .line_height(px(geometry.subtext_line))
            .text_color(ink(kind, tokens))
            .whitespace_normal()
            .when(kind == FeedbackKind::Error, |line| line.line_clamp(1))
            .child(text.clone())
    });
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(px(geometry.gap))
        .child(header)
        .child(content.body)
        .children(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scheme;
    #[test]
    fn source_geometry_and_feedback_ink_hold_in_both_themes() {
        assert_eq!(geometry().gap.to_bits(), 8.0_f32.to_bits());
        // Metrics use the same global UI scale as the existing fields.
        assert_eq!(geometry().subtext_line.to_bits(), fonts::ui(17.0).to_bits());
        assert_eq!(Type::Body.metrics().0.to_bits(), fonts::ui(14.0).to_bits());
        assert_eq!(Type::Body.metrics().2, gpui::FontWeight::NORMAL);
        assert_eq!(Type::Meta.metrics().0.to_bits(), fonts::ui(12.0).to_bits());
        for scheme in [Scheme::Dark, Scheme::Light] {
            let tokens = Tokens::of(scheme);
            assert_eq!(
                ink(FeedbackKind::Hint, &tokens),
                tokens.neutral.foreground_muted
            );
            assert_eq!(
                ink(FeedbackKind::Error, &tokens),
                gpui::rgb(0x00fc_a5a5).into()
            );
        }
    }
    #[test]
    fn error_is_the_only_feedback_when_hint_is_also_supplied() {
        for (hint, error) in [
            ("Choose a project", "Choose a ready project"),
            ("프로젝트를 선택하세요", "준비된 프로젝트를 선택하세요"),
        ] {
            let hint: SharedString = hint.into();
            let error: SharedString = error.into();
            assert_eq!(
                feedback(Some(&hint), Some(&error)),
                Some((FeedbackKind::Error, &error))
            );
            assert_eq!(
                feedback(Some(&hint), None),
                Some((FeedbackKind::Hint, &hint))
            );
            assert_eq!(
                feedback(None, Some(&error)),
                Some((FeedbackKind::Error, &error))
            );
            assert_eq!(feedback(None, None), None);
            let blank: SharedString = "".into();
            assert_eq!(
                feedback(Some(&hint), Some(&blank)),
                Some((FeedbackKind::Hint, &hint))
            );
            assert_eq!(feedback(Some(&blank), None), None);
        }
    }
}
