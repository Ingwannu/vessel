//! Literal source slot, plus real wrapper/child bounds on both themes.
use super::{FORM_ROW_EXTENT, ROW_GLYPH, form_glyph, leading_width};
use crate::{Scheme, SchemeChoice, Tokens, assets::Glyph, picker::Leading};
use gpui::prelude::*;
use gpui::{Bounds, Pixels, Render, TestAppContext};
use std::{cell::RefCell, rc::Rc};
#[test]
fn form_slot_is_opt_in_and_does_not_resize_legacy_leading() {
    assert_eq!(FORM_ROW_EXTENT.to_bits(), 18.0_f32.to_bits());
    assert_eq!(ROW_GLYPH.to_bits(), 16.0_f32.to_bits());
    assert_eq!(
        leading_width(&Leading::FormGlyph(Glyph::Folder)),
        Some(18.0)
    );
    assert_eq!(leading_width(&Leading::Glyph(Glyph::Folder)), Some(16.0));
    assert_eq!(
        leading_width(&Leading::SmallGlyph(Glyph::Folder)),
        Some(14.0)
    );
    for scheme in [Scheme::Dark, Scheme::Light] {
        let mut frame = form_glyph(Glyph::Brain, &Tokens::of(scheme));
        assert_eq!(frame.style().size.width, Some(gpui::px(18.0).into()));
        assert_eq!(frame.style().size.height, Some(gpui::px(18.0).into()));
    }
}
struct Probe {
    scheme: Scheme,
    bounds: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let bounds = Rc::clone(&self.bounds);
        form_glyph(Glyph::Folder, &Tokens::of(self.scheme))
            .on_children_prepainted(move |children, _, _| *bounds.borrow_mut() = children)
    }
}
#[gpui::test]
fn actual_form_glyph_keeps_sixteen_point_child_in_eighteen_point_frame(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let initialized = crate::init(cx, SchemeChoice::Fixed(Scheme::Dark));
        assert!(initialized.is_ok());
    });
    for scheme in [Scheme::Dark, Scheme::Light] {
        let bounds = Rc::new(RefCell::new(Vec::new()));
        let probe_bounds = Rc::clone(&bounds);
        let (_view, cx) = cx.add_window_view(|_, _| Probe {
            scheme,
            bounds: probe_bounds,
        });
        cx.run_until_parked();
        let seen = bounds.borrow();
        assert_eq!(seen.len(), 1);
        let Some(child) = seen.first() else {
            return;
        };
        assert_eq!(child.size.width, gpui::px(16.0));
        assert_eq!(child.size.height, gpui::px(16.0));
    }
}
