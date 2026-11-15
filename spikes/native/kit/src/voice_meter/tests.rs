//! Measure the real shared bar constructors in GPUI for both themes.
use super::*;
use crate::Scheme;
use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window, canvas, point, size};
use std::{cell::RefCell, rc::Rc};

type BoundsSeen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    scheme: Scheme,
    state: State,
    seen: BoundsSeen,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let geometry = Geometry::of(self.state);
        let children = lines(geometry, tokens.neutral.foreground).map(|line| {
            let seen = Rc::clone(&self.seen);
            line.child(
                canvas(
                    move |bounds, _, _| seen.borrow_mut().push(bounds),
                    |_, (), _, _| {},
                )
                .size_full(),
            )
        });
        frame().w(px(40.0)).children(children)
    }
}

#[gpui::test]
fn horizontal_bars_keep_reference_geometry_in_both_themes(cx: &mut TestAppContext) {
    let full = Level::new(1.0).unwrap();
    // GPUI rounds layout origins half toward zero (taffy.rs), while
    // retaining the reference fractional bar heights.
    let cases = [
        (State::Muted, [12.0, 12.0, 12.0], [24.0, 24.0, 24.0]),
        (State::Listening(full), [48.0, 43.5, 39.0], [6.0, 8.0, 10.0]),
        (State::Speaking(full), [48.0, 43.5, 39.0], [6.0, 8.0, 10.0]),
    ];
    for scheme in [Scheme::Light, Scheme::Dark] {
        for (state, heights, positions) in cases {
            let seen = BoundsSeen::default();
            let _window = cx.open_window(size(px(40.0), px(60.0)), {
                let seen = Rc::clone(&seen);
                move |_, _| Probe {
                    scheme,
                    state,
                    seen,
                }
            });
            cx.run_until_parked();
            let actual = seen.borrow();
            assert_eq!(actual.len(), 3);
            for ((bounds, (height, y)), x) in actual
                .iter()
                .zip(heights.into_iter().zip(positions))
                .zip([0.0, 16.0, 32.0])
            {
                assert_eq!(
                    *bounds,
                    Bounds::new(point(px(x), px(y)), size(px(8.0), px(height)))
                );
            }
        }
    }
}

#[test]
fn invalid_levels_cannot_draw_and_mute_or_maximum_speech_has_bounded_alpha() {
    for raw in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        assert!(Level::new(raw).is_err());
    }
    let muted = Geometry::of(State::Muted);
    assert_eq!(muted.heights.map(f32::to_bits), [12.0_f32.to_bits(); 3]);
    assert_eq!(muted.opacity.to_bits(), 0.3_f32.to_bits());
    assert_eq!(
        Geometry::of(State::Speaking(Level::new(1.0).unwrap()))
            .opacity
            .to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(
        motion::transition(motion::QUICK).duration,
        std::time::Duration::from_millis(150)
    );
}
