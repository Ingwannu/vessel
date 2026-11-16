//! Source styles and the existing input/identity route; no OS-font/IME claim.
use super::*;
use crate::{Scheme, SchemeChoice, Tokens};
use gpui::prelude::*;
use gpui::{FontWeight, Render, TestAppContext, div, px};
#[test]
fn code_and_body_keep_the_source_form_chrome_in_both_schemes_and_layouts() {
    assert_eq!(FormText::default(), FormText::Body);
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        for (narrow, height) in [(false, 32.0), (true, 44.0)] {
            for (text, family) in [
                (FormText::Body, Type::Body.family()),
                (FormText::Code, Type::MonoBody.family()),
            ] {
                let mut frame = text.apply(super::super::form_shell(false, &tokens, narrow));
                let style = frame.style();
                assert_eq!(style.min_size.height, Some(px(height).into()));
                assert_eq!(style.padding.left, Some(px(12.0).into()));
                assert_eq!(style.corner_radii.top_left, Some(px(8.0).into()));
                assert_eq!(style.background, Some(tokens.neutral.surface2.into()));
                assert_eq!(style.border_widths.left, Some(px(1.0).into()));
                assert_eq!(style.text.font_family, Some(family));
                assert_eq!(style.text.font_size, Some(px(14.0).into()));
                assert_eq!(style.text.line_height, Some(px(20.0).into()));
                assert_eq!(style.text.font_weight, Some(FontWeight::NORMAL));
                assert_eq!(style.text.color, Some(tokens.neutral.foreground));
            }
        }
    }
}
struct Probe {
    field: TextField,
    scheme: Scheme,
    narrow: bool,
    changed: Vec<(FieldId, String)>,
    submitted: Option<(FieldId, String)>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().child(
            self.field
                .element(&Tokens::of(self.scheme), self.narrow, cx),
        )
    }
}
#[gpui::test]
fn code_form_uses_the_same_engine_and_reports_its_retained_source(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let initialized = crate::init(cx, SchemeChoice::Fixed(Scheme::Dark));
        assert!(initialized.is_ok());
    });
    for scheme in [Scheme::Light, Scheme::Dark] {
        for narrow in [false, true] {
            let (view, cx) = cx.add_window_view(|window, cx| Probe {
                field: TextField::with_source_text(
                    "0 9 * * *",
                    |owner: &mut Probe, id, text, _, _| owner.submitted = Some((id, text)),
                    |owner: &mut Probe, id, text, _| owner.changed.push((id, text)),
                    FormText::Code,
                    window,
                    cx,
                ),
                scheme,
                narrow,
                changed: Vec::new(),
                submitted: None,
            });
            cx.update(|window, _| window.activate_window());
            view.update_in(cx, |probe, window, cx| probe.field.focus(window, cx));
            cx.run_until_parked();
            cx.simulate_input("0 9");
            cx.simulate_keystrokes("enter");
            cx.run_until_parked();
            view.update_in(cx, |probe, _, cx| {
                let id = probe.field.id();
                assert_eq!(probe.field.form_text, FormText::Code);
                assert_eq!(probe.field.text(cx), "0 9");
                assert_eq!(probe.field.clone().id(), id);
                assert_eq!(
                    probe.changed,
                    [
                        (id, "0".to_owned()),
                        (id, "0 ".to_owned()),
                        (id, "0 9".to_owned()),
                        // simulate_input dispatches a final empty keystroke;
                        // the existing engine reports that unchanged value too.
                        (id, "0 9".to_owned())
                    ]
                );
                assert_eq!(probe.submitted, Some((id, "0 9".to_owned())));
            });
        }
    }
}
