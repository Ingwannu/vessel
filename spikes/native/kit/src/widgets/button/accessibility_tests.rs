//! Every kit button is announced as a button named by its words, as Paseo's
//! `Button` (`accessibilityRole="button"`); a pending one keeps its name, and
//! an icon-only one is named by its caller (TRACKER 782).
use super::*;
use crate::a11y_snapshot::{activate, button_names};
use crate::assets::Glyph;
use crate::{Scheme, SchemeChoice};
use gpui::{Render, TestAppContext};

struct Probe;

impl Render for Probe {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let pending = ButtonConfig::new(
            ButtonKind::Primary,
            ButtonState::Enabled,
            ControlSizing::Pointer,
        )
        .loading();
        div()
            .flex()
            .flex_col()
            .child(button_with_kind(
                "save",
                "Save",
                ButtonKind::Primary,
                ButtonState::Enabled,
                &tokens,
            ))
            .child(button_control(
                "apply",
                ButtonContent::Label("Apply".into()),
                pending,
                &tokens,
            ))
            .child(button_with_content(
                "unnamed",
                ButtonContent::Icon(ButtonIcon::new(Glyph::Stop)),
                ButtonKind::Ghost,
                ButtonState::Enabled,
                ControlSizing::Pointer,
                &tokens,
            ))
            .child(
                button_control(
                    "stop",
                    ButtonContent::Icon(ButtonIcon::new(Glyph::Stop)),
                    ButtonConfig::new(
                        ButtonKind::Ghost,
                        ButtonState::Enabled,
                        ControlSizing::Pointer,
                    ),
                    &tokens,
                )
                .aria_label("Stop"),
            )
            .child(crate::outline_button("remove", "Remove", true, &tokens))
    }
}

#[gpui::test]
fn kit_buttons_are_buttons_named_by_their_words(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|_, _| Probe);
    cx.run_until_parked();
    activate(cx);
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    let names = cx.update(|window, _| button_names(window).unwrap());
    // The icon-only control its caller did not name is still a button: the
    // gap is visible here, not hidden.
    assert_eq!(
        names,
        vec![
            None,
            Some("Apply".to_owned()),
            Some("Remove".to_owned()),
            Some("Save".to_owned()),
            Some("Stop".to_owned()),
        ]
    );
}
