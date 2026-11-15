use super::*;
use crate::{Scheme, assets::Assets};
use gpui::{
    AssetSource as _, Bounds, Context, Modifiers, Pixels, Render, TestAppContext, Window, point,
};
use std::{cell::RefCell, rc::Rc};

#[test]
fn source_geometry_and_own_hover_ink_have_literal_values_in_both_themes() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        assert_eq!(px(ARROW), px(12.0));
        assert_eq!(px(GAP), px(4.0));
        assert_eq!(px(SLOP), px(8.0));
        assert_eq!(px(Type::Meta.metrics().0), px(12.0));
        assert_eq!(
            ink(&tokens),
            Ink {
                idle: tokens.neutral.foreground_muted,
                hover: tokens.neutral.foreground
            }
        );
        let mut link = external_link("literal-link", Some("Docs".into()), &tokens);
        let style = link.style();
        assert_eq!(style.padding.left, Some(px(8.0).into()));
        assert_eq!(style.padding.right, Some(px(8.0).into()));
        assert_eq!(style.padding.top, Some(px(8.0).into()));
        assert_eq!(style.padding.bottom, Some(px(8.0).into()));
        assert_eq!(style.margin.left, Some(px(-8.0).into()));
        assert_eq!(style.margin.right, Some(px(-8.0).into()));
        assert_eq!(style.margin.top, Some(px(-8.0).into()));
        assert_eq!(style.margin.bottom, Some(px(-8.0).into()));
        assert_eq!(style.background, None);
    }
}

#[test]
fn arrow_is_one_registered_unclipped_original_twelve_pixel_mask() {
    assert_eq!(Glyph::ArrowUpRight.path(), "kit/arrow-up-right.svg");
    let bytes = Assets.load(Glyph::ArrowUpRight.path()).unwrap().unwrap();
    let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
    assert_eq!(px(tree.size().width()), px(24.0));
    assert_eq!(px(tree.size().height()), px(24.0));
    let side = 12;
    let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(0.5, 0.5),
        &mut pixels.as_mut(),
    );
    let image = image::RgbaImage::from_raw(side, side, pixels.take()).unwrap();
    assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
    for at in 0..side {
        for (x, y) in [(0, at), (11, at), (at, 0), (at, 11)] {
            assert_eq!(image.get_pixel(x, y).0[3], 0);
        }
    }
}

type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    scheme: Scheme,
    seen: Seen,
    clicks: usize,
    parent: usize,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let seen = Rc::clone(&self.seen);
        div().size_full().p(px(24.0)).child(
            div()
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
                .id("parent")
                .on_click(cx.listener(|this, _, _, _| this.parent = this.parent.saturating_add(1)))
                .flex()
                .flex_col()
                .items_start()
                .child(
                    external_link("docs", Some("Docs".into()), &tokens).on_click(
                        cx.listener(|this, _, _, _| this.clicks = this.clicks.saturating_add(1)),
                    ),
                )
                .child(external_link("icon", None, &tokens)),
        )
    }
}
#[gpui::test]
fn expanded_target_changes_input_bounds_but_keeps_visual_row_pitch(cx: &mut TestAppContext) {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let seen = Seen::default();
        let (view, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            move |_, _| Probe {
                scheme,
                seen,
                clicks: 0,
                parent: 0,
            }
        });
        cx.run_until_parked();
        let recorded = seen.borrow();
        let [label, icon] = recorded.as_slice() else {
            panic!("two actual link targets")
        };
        let label = *label;
        let icon = *icon;
        drop(recorded);
        assert_eq!(label.size.height, px(32.0)); // Meta16 plus8 on both sides.
        assert_eq!(icon.size.height, px(28.0)); // Arrow12 plus8 on both sides.
        assert_eq!(icon.size.width, px(28.0));
        assert_eq!(icon.origin.y - label.origin.y, px(16.0));
        cx.simulate_click(
            point(label.left() + px(1.0), label.top() + px(1.0)),
            Modifiers::none(),
        );
        cx.run_until_parked();
        view.read_with(cx, |this, _| assert_eq!((this.clicks, this.parent), (1, 0)));
    }
}
