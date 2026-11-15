//! Rendered bounds distinguish configured width/fixed height from legacy fitting.

use super::*;
use crate::sheet::snap::{Percentage, SnapPoints};
use crate::{Scheme, SchemeChoice};
use gpui::{Bounds, Pixels, Render, TestAppContext, size};
use std::{cell::RefCell, rc::Rc};

struct Probe {
    state: DialogState,
    narrow: bool,
    body: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    footer: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let body = Rc::clone(&self.body);
        let footer = Rc::clone(&self.footer);
        let height = Percentage::new(85).unwrap();
        let points = SnapPoints::new(85, 95).unwrap();
        let layout = DialogLayout::new(
            620,
            DesktopHeight::Exact(height),
            CompactLayout::Draggable(points),
        )
        .unwrap();
        dialog(
            &self.state,
            DialogContent {
                title: "Release history".into(),
                body: div()
                    .w_full()
                    .on_children_prepainted(move |bounds, _, _| *body.borrow_mut() = bounds)
                    .child(div().w_full().h(px(1.0)).flex_shrink_0())
                    .into_any_element(),
                footer: vec![
                    div()
                        .on_children_prepainted(move |bounds, _, _| *footer.borrow_mut() = bounds)
                        .child(div().w(px(80.0)).h(px(32.0)))
                        .into_any_element(),
                ],
                size: DialogSize::Configured(layout),
            },
            &Tokens::of(Scheme::Dark),
            window,
            self.narrow,
            |_, _, _| {},
            cx,
        )
    }
}

#[gpui::test]
fn configured_dialog_uses_620_width_and_full_percent_heights_for_short_content(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        cx.set_reduce_motion(true);
    });
    for (width, narrow, expected_body_width) in [(1280.0, false, 570.0), (390.0, true, 342.0)] {
        let body = Rc::new(RefCell::new(Vec::new()));
        let footer = Rc::new(RefCell::new(Vec::new()));
        let (_, window) = cx.add_window_view({
            let body = Rc::clone(&body);
            let footer = Rc::clone(&footer);
            move |_, cx| Probe {
                state: DialogState::new(cx),
                narrow,
                body,
                footer,
            }
        });
        window.simulate_resize(size(px(width), px(900.0)));
        window.run_until_parked();
        let body = body.borrow();
        let body = body.first().expect("body marker laid out");
        let footer = footer.borrow();
        let footer = footer.first().expect("footer marker laid out");
        assert_eq!(body.size.width, px(expected_body_width));
        assert!(
            f32::from(body.top()).mul_add(-1.0, f32::from(footer.top())) > 400.0,
            "short content must not collapse the authored percent height"
        );
    }
}

#[test]
fn legacy_sizes_retain_natural_height_caps_and_invalid_width_is_refused() {
    for (size, width) in [(DialogSize::Normal, 520), (DialogSize::Wide, 640)] {
        let layout = size.layout();
        assert_eq!(layout.width().get(), width);
        let DesktopHeight::AtMost(height) = layout.height() else {
            panic!("legacy fitting height");
        };
        assert_eq!(height.value(), 85);
        let CompactLayout::Fixed(height) = layout.compact() else {
            panic!("legacy fitting compact sheet");
        };
        assert_eq!(height.value(), 90);
    }
    assert!(
        DialogLayout::new(
            0,
            DesktopHeight::Exact(Percentage::FULL),
            CompactLayout::Fixed(Percentage::FULL)
        )
        .is_none()
    );
}
