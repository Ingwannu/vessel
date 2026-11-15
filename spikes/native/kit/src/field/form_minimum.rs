//! Optional source minimum on the existing textarea; old GOAT68/80 defaults stay.
//!
//! [Decision Log]
//! - Purpose: schedule-form-sheet.tsx1040 explicitly sets minHeight96.
//! - Constraints: same `TextareaState`, callbacks, `Editability` and existing cap; no area
//!   `.min_h` override or parallel editor; construction stays in its engine.
//! - Choice: a closed typed minimum supplied only by a multiline-form constructor.
//! - Why/cost: no arbitrary/nonfinite dimension, no option silently ignored on a
//!   composer/line. The old `form_shell` moves here unchanged to lower the ratchet.
use super::{FORM_MIN, FORM_MIN_TOUCH, TextField};
use crate::{Tokens, Type, TypeExt as _};
use gpui::prelude::*;
use gpui::{Context, Window, div, px};

const EXPANDED_MINIMUM: f32 = crate::theme::spacing::S24;
/// The two verified source minima for the existing form textarea.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FormMinimum {
    /// Existing GOAT68 pointer/80 touch, preserving every old caller.
    #[default]
    Default,
    /// Source96 in both layouts (Paseo's scheduled prompt field).
    Expanded,
}
impl FormMinimum {
    pub(super) const fn height(self, narrow: bool) -> f32 {
        match self {
            Self::Default => {
                if narrow {
                    FORM_MIN_TOUCH
                } else {
                    FORM_MIN
                }
            }
            Self::Expanded => EXPANDED_MINIMUM,
        }
    }
}
impl TextField {
    /// Existing multiline form with an explicit source minimum, same engine.
    /// Enter/IME/editability/callback behavior and maximum remain unchanged.
    #[must_use]
    pub fn form_multiline_with_minimum<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        minimum: FormMinimum,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let mut field = Self::form_multiline(placeholder, on_submit, window, cx);
        field.form_minimum = minimum;
        field
    }
}

/// Both form editors use the same surface and border-only focus treatment.
pub(super) fn form_shell(focused: bool, tokens: &Tokens, narrow: bool) -> gpui::Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .items_center()
        .w_full()
        .min_h(px(if narrow { 44.0 } else { 32.0 }))
        .px(px(12.0))
        .rounded(px(8.0))
        .bg(neutral.surface2)
        .border_1()
        .border_color(if focused {
            neutral.border_accent
        } else {
            gpui::transparent_black()
        })
        .hover(move |style| style.border_color(neutral.border_accent))
        .type_(Type::Body)
        .line_height(px(crate::fonts::ui(20.0)))
        .text_color(neutral.foreground)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Bounds, IntoElement, Pixels, Render, TestAppContext};
    use std::{cell::RefCell, rc::Rc};
    #[test]
    fn legacy_and_expanded_minimum_values_are_literal_in_both_layouts() {
        assert_eq!(FormMinimum::default(), FormMinimum::Default);
        assert_eq!(
            FormMinimum::Default.height(false).to_bits(),
            68.0_f32.to_bits()
        );
        assert_eq!(
            FormMinimum::Default.height(true).to_bits(),
            80.0_f32.to_bits()
        );
        for narrow in [false, true] {
            assert_eq!(
                FormMinimum::Expanded.height(narrow).to_bits(),
                96.0_f32.to_bits()
            );
        }
    }
    struct Probe {
        field: TextField,
        narrow: bool,
        tokens: Tokens,
        seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
            let seen = Rc::clone(&self.seen);
            div()
                .w(px(360.0))
                .flex()
                .flex_col()
                .items_start()
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
                .child(self.field.element(&self.tokens, self.narrow, cx))
        }
    }
    #[gpui::test]
    fn expanded_uses_the_real_form_engine_at96_and_keeps_the_existing_cap(cx: &mut TestAppContext) {
        cx.update(gpui_base::init);
        for scheme in [crate::Scheme::Light, crate::Scheme::Dark] {
            for narrow in [false, true] {
                let seen = Rc::new(RefCell::new(Vec::new()));
                let (view, cx) = cx.add_window_view({
                    let seen = Rc::clone(&seen);
                    move |window, cx| Probe {
                        field: TextField::form_multiline_with_minimum(
                            "Prompt",
                            |_, _, _, _| {},
                            FormMinimum::Expanded,
                            window,
                            cx,
                        ),
                        narrow,
                        tokens: Tokens::of(scheme),
                        seen,
                    }
                });
                cx.run_until_parked();
                assert_eq!(seen.borrow()[0].size.height, px(96.0));
                cx.update(|window, cx| {
                    view.update(cx, |probe, cx| {
                        probe
                            .field
                            .set_text(&vec!["line"; 30].join("\n"), window, cx);
                        cx.notify();
                    });
                });
                cx.run_until_parked();
                let height = seen.borrow()[0].size.height;
                assert!(height > px(96.0));
                assert!(height <= px(super::super::FORM_MAX));
            }
        }
    }
}
