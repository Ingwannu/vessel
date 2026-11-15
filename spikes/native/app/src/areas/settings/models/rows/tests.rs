//! Long model identities must stay readable beside a real copy target.
use super::*;
use gpui::{Bounds, Modifiers, Pixels, Render, TestAppContext};
use std::{cell::RefCell, rc::Rc};

type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    width: f32,
    compact: bool,
    value: String,
    seen: Seen,
}
impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let tokens = Tokens::of(vessel_app_kit::Scheme::Dark);
        let seen = Rc::clone(&self.seen);
        div().w(px(self.width)).p(px(spacing::S4)).child(
            identifier("model", &self.value, &tokens, self.compact, window, cx)
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds),
        )
    }
}
#[gpui::test]
fn models_long_identifiers_keep_a_whole_copy_target_inside_both_widths(cx: &mut TestAppContext) {
    for (width, compact, right) in [(390.0, true, 374.0), (1280.0, false, 1264.0)] {
        for value in [
            "unbrokenmodelidentifier".repeat(20),
            "fixture/긴-한글-모델-식별자/".repeat(20),
        ] {
            let seen = Seen::default();
            let window = cx.open_window(gpui::size(px(width), px(844.0)), {
                let value = value.clone();
                let seen = Rc::clone(&seen);
                move |_, _| Probe {
                    width,
                    compact,
                    value,
                    seen,
                }
            });
            let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);
            visual.run_until_parked();
            let children = seen.borrow().clone();
            assert_eq!(children.len(), 2);
            let (text, copy) = (children[0], children[1]);
            assert!(text.right() <= copy.left(), "text={text:?}, copy={copy:?}");
            assert!(copy.left() >= px(spacing::S4), "{copy:?}");
            assert!(copy.right() <= px(right), "{copy:?}");
            if compact {
                assert_eq!(
                    copy.size.width,
                    px(vessel_app_kit::breakpoint::TOUCH_TARGET)
                );
                assert_eq!(
                    copy.size.height,
                    px(vessel_app_kit::breakpoint::TOUCH_TARGET)
                );
            }
            visual.simulate_click(copy.center(), Modifiers::default());
            assert_eq!(
                visual.read_from_clipboard().and_then(|item| item.text()),
                Some(value)
            );
        }
    }
}
