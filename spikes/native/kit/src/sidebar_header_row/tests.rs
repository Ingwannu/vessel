use super::*;
use crate::Scheme;
use gpui::{Bounds, Context, Modifiers, Pixels, Render, TestAppContext, point, size};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    variant: Variant,
    density: Density,
    seen: Seen,
    clicks: Rc<Cell<usize>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let seen = Rc::clone(&self.seen);
        let clicks = Rc::clone(&self.clicks);
        div().w(px(320.0)).flex().flex_col().child(
            SidebarHeaderRow::new("source-nav", "Search", self.variant)
                .leading(Glyph::Search)
                .element(self.density, &Tokens::of(Scheme::Dark), move |_, _, _| {
                    clicks.set(clicks.get() + 1)
                })
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds),
        )
    }
}
#[gpui::test]
fn actual_rows_match_reference_wrapper_density_and_receive_one_press(cx: &mut TestAppContext) {
    for (density, variant, x, y, w, h) in [
        (Density::Desktop, Variant::Compact, 8.0, 0.0, 304.0, 28.0),
        (Density::Desktop, Variant::Inline, 0.0, 0.0, 320.0, 28.0),
        (Density::Desktop, Variant::Header, 8.0, 3.5, 304.0, 28.0),
        (Density::Compact, Variant::Compact, 8.0, 0.0, 304.0, 36.0),
        (Density::Compact, Variant::Inline, 0.0, 0.0, 320.0, 36.0),
        (Density::Compact, Variant::Header, 8.0, 9.5, 304.0, 36.0),
    ] {
        let seen = Seen::default();
        let clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            let clicks = Rc::clone(&clicks);
            move |_, _| Probe {
                variant,
                density,
                seen,
                clicks,
            }
        });
        cx.run_until_parked();
        let expected = Bounds::new(point(px(x), px(y)), size(px(w), px(h)));
        assert_eq!(*seen.borrow(), vec![expected]);
        cx.simulate_click(expected.center(), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(clicks.get(), 1);
    }
}
#[test]
fn selection_hover_and_press_use_source_tokens_with_press_precedence() {
    for scheme in [Scheme::Dark, Scheme::Light] {
        let tokens = Tokens::of(scheme);
        let hover = match scheme {
            Scheme::Dark => tokens.neutral.surface1,
            Scheme::Light => tokens.neutral.surface3,
        };
        assert_eq!(
            paint(Selection::Inactive, Phase::Idle, &tokens).fill,
            gpui::transparent_black()
        );
        assert_eq!(
            paint(Selection::Inactive, Phase::Idle, &tokens).ink,
            tokens.neutral.foreground_muted
        );
        for selection in [Selection::Inactive, Selection::Active] {
            assert_eq!(paint(selection, Phase::Hovered, &tokens).fill, hover);
            assert_eq!(
                paint(selection, Phase::Hovered, &tokens).ink,
                tokens.neutral.foreground
            );
            assert_eq!(
                paint(selection, Phase::Pressed, &tokens).fill,
                tokens.neutral.surface2
            );
        }
        assert_eq!(paint(Selection::Active, Phase::Idle, &tokens).fill, hover);
    }
}
