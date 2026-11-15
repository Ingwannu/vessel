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
                    clicks.set(clicks.get().saturating_add(1));
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

type Place = Rc<Cell<Option<Bounds<Pixels>>>>;
struct SlotsProbe {
    persistent: bool,
    hint: Place,
    trailing: Place,
    row_presses: Rc<Cell<usize>>,
    child_presses: Rc<Cell<usize>>,
}
fn slot(place: &Place) -> gpui::Div {
    div()
        .relative()
        .w(px(40.0))
        .h(px(20.0))
        .child(crate::picker::anchor_probe(place))
}
impl Render for SlotsProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let row_presses = Rc::clone(&self.row_presses);
        let mut row = SidebarHeaderRow::new("slot-nav", "Search", Variant::Compact)
            .leading(Glyph::Search)
            .shortcut(slot(&self.hint).into_any_element());
        if self.persistent {
            let child_presses = Rc::clone(&self.child_presses);
            row = row.trailing(
                slot(&self.trailing)
                    .id("slot-child")
                    .on_click(move |_, _, cx| {
                        child_presses.set(child_presses.get().saturating_add(1));
                        cx.stop_propagation();
                    })
                    .into_any_element(),
            );
        }
        div().w(px(320.0)).child(row.element(
            Density::Desktop,
            &Tokens::of(Scheme::Dark),
            move |_, _, _| row_presses.set(row_presses.get().saturating_add(1)),
        ))
    }
}
#[gpui::test]
fn sidebar_hint_occupies_no_idle_slot_and_a_persistent_child_takes_precedence(
    cx: &mut TestAppContext,
) {
    for persistent in [false, true] {
        let hint = Place::default();
        let trailing = Place::default();
        let row_presses = Rc::new(Cell::new(0));
        let child_presses = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let hint = Rc::clone(&hint);
            let trailing = Rc::clone(&trailing);
            let row_presses = Rc::clone(&row_presses);
            let child_presses = Rc::clone(&child_presses);
            move |_, _| SlotsProbe {
                persistent,
                hint,
                trailing,
                row_presses,
                child_presses,
            }
        });
        cx.run_until_parked();
        assert!(hint.get().is_none());
        assert_eq!(trailing.get().is_some(), persistent);
        cx.simulate_mouse_move(point(px(20.0), px(14.0)), None, Modifiers::none());
        cx.run_until_parked();
        let expected = Bounds::new(point(px(264.0), px(4.0)), size(px(40.0), px(20.0)));
        if persistent {
            assert!(hint.get().is_none());
            assert_eq!(trailing.get(), Some(expected));
        } else {
            assert_eq!(hint.get(), Some(expected));
        }
        cx.simulate_click(expected.center(), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(row_presses.get(), usize::from(!persistent));
        assert_eq!(child_presses.get(), usize::from(persistent));
        hint.set(None);
        cx.simulate_mouse_move(point(px(350.0), px(50.0)), None, Modifiers::none());
        cx.run_until_parked();
        assert!(hint.get().is_none());
    }
}
