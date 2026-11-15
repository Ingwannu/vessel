//! Bounds from the actual renderer, against the authored `ThemePicker` geometry.
use super::*;
use crate::Scheme;
use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window};
use std::{cell::RefCell, rc::Rc};

type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
const EPSILON: f32 = 0.02;

struct Probe {
    scheme: Scheme,
    state: State,
    split: bool,
    children: Seen,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let light = Tokens::of(Scheme::Light);
        let dark = Tokens::of(Scheme::Dark);
        let preview = if self.split {
            Preview::Split {
                light: &light,
                dark: &dark,
            }
        } else {
            Preview::Single(&tokens)
        };
        let seen = Rc::clone(&self.children);
        frame(
            "A very long theme name that must truncate",
            self.state,
            preview,
            &tokens,
        )
        .w(px(200.0))
        .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
        .id("theme-tile-probe")
    }
}

fn near(actual: Pixels, expected: f32) {
    assert!(
        expected.mul_add(-1.0, f32::from(actual)).abs() < EPSILON,
        "{actual:?} != {expected}"
    );
}

#[gpui::test]
fn single_and_split_keep_the_reference_frame_and_small_label_in_both_schemes(
    cx: &mut TestAppContext,
) {
    // Independently calculated physical-edge expectations. GPUI snaps authored
    // borders/padding before aspect layout and then snaps absolute edges;
    // half pixels round toward zero (taffy.rs), including the 2px border at
    // 125%. Compare exact projected bounds rather than loosening tolerance.
    for (scale, origin, width, height, label_y) in [
        (1.0, 6.0, 188.0, 107.0, 117.0),
        (1.25, 5.6, 188.8, 107.2, 116.8),
        (1.5, 6.0, 188.0, 106.0, 116.0),
        (2.0, 6.0, 188.0, 106.5, 116.5),
    ] {
        for scheme in [Scheme::Light, Scheme::Dark] {
            for state in [State::Idle, State::Chosen] {
                for split in [false, true] {
                    let children = Seen::default();
                    let (_, cx) = cx.add_window_view({
                        let children = Rc::clone(&children);
                        move |_, _| Probe {
                            scheme,
                            state,
                            split,
                            children,
                        }
                    });
                    cx.simulate_scale_factor_change(scale);
                    cx.run_until_parked();
                    let children = children.borrow();
                    assert_eq!(children.len(), 2);
                    let picture = children[0];
                    let label = children[1];
                    near(picture.origin.x, origin);
                    near(picture.origin.y, origin);
                    near(picture.size.width, width);
                    near(picture.size.height, height);
                    near(label.origin.x, origin);
                    near(label.origin.y, label_y);
                    near(label.size.width, width);
                    near(label.size.height, 16.0);
                }
            }
        }
    }
}

#[test]
fn the_grid_changes_at_the_reference_viewport_breakpoint() {
    assert_eq!(Columns::at(px(599.0)), Columns::Two);
    assert_eq!(Columns::at(px(600.0)), Columns::Four);
}
