//! Easing values are fixtures from Paseo's installed react-native-web
//! Animated/Easing + `TimingAnimation` default. The GPUI probe observes
//! actual shape alpha, geometry and mounted animation scheduling.
use super::*;
use crate::Scheme;
use gpui::{Bounds, Context, Render, TestAppContext, Window, WindowHandle, point, size};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn the_two_second_pulse_matches_reference_timing_in_both_directions() {
    let cases: &[(f32, f64)] = &[
        (0.0, 0.4),
        (0.0625, 0.418_692_930_143_765),
        (0.125, 0.463_071_362_514_507_87),
        (0.25, 0.6),
        (0.375, 0.736_928_637_485_492_2),
        (0.4375, 0.781_307_069_856_235_1),
        (0.5, 0.8),
        (0.625, 0.736_928_637_485_492_2),
        (0.75, 0.6),
        (0.875, 0.463_071_362_514_507_87),
        (1.0, 0.4),
    ];
    for &(phase, expected) in cases {
        let definition = motion();
        assert_eq!(definition.duration, Duration::from_secs(2));
        assert!(!definition.oneshot && !definition.synced);
        let actual = f64::from(Opacity::from_eased((definition.easing)(phase)).0);
        assert!(
            expected.mul_add(-1.0, actual).abs() < 0.000_001,
            "phase {phase}: {actual} vs {expected}"
        );
    }
    assert_eq!(Rounding::Small.radius(), px(2.0));
    assert_eq!(Rounding::Medium.radius(), px(6.0));
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sample {
    left: Option<f32>,
    right: Option<f32>,
    frame: Option<f32>,
}
type Samples = Rc<RefCell<Vec<Sample>>>;
type Geometry = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    mounted: bool,
    phase: Rc<Cell<f32>>,
    samples: Samples,
    geometry: Geometry,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        if !self.mounted {
            return div().into_any_element();
        }
        let samples = Rc::clone(&self.samples);
        let geometry = Rc::clone(&self.geometry);
        let phase = Rc::clone(&self.phase);
        animated(
            "one-skeleton",
            move |opacity| {
                let tokens = Tokens::of(Scheme::Dark);
                let mut left = shape(
                    Shape {
                        size: size(px(20.0), px(20.0)),
                        rounding: Rounding::Medium,
                    },
                    opacity,
                    &tokens,
                );
                let mut right = shape(
                    Shape {
                        size: size(px(160.0), px(12.0)),
                        rounding: Rounding::Small,
                    },
                    opacity,
                    &tokens,
                );
                let mut parent = div()
                    .w(px(200.0))
                    .h(px(80.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0));
                samples.borrow_mut().push(Sample {
                    left: left.style().opacity,
                    right: right.style().opacity,
                    frame: parent.style().opacity,
                });
                let geometry = Rc::clone(&geometry);
                parent
                    .child(left)
                    .child(right)
                    .on_children_prepainted(move |bounds, _, _| *geometry.borrow_mut() = bounds)
            },
            motion().with_easing(move |_| phase.get()),
        )
    }
}
fn frame(window: &WindowHandle<Probe>, cx: &mut TestAppContext) -> usize {
    let callbacks = window
        .update(cx, |_, window, cx| window.simulate_next_frame(cx))
        .unwrap();
    cx.run_until_parked();
    callbacks
}
#[gpui::test]
fn shapes_share_one_phase_furniture_stays_solid_and_unmount_stops_it(cx: &mut TestAppContext) {
    let samples = Samples::default();
    let geometry = Geometry::default();
    let phase = Rc::new(Cell::new(0.0));
    let window = cx.open_window(size(px(256.0), px(80.0)), {
        let samples = Rc::clone(&samples);
        let geometry = Rc::clone(&geometry);
        let phase = Rc::clone(&phase);
        move |_, _| Probe {
            mounted: true,
            phase,
            samples,
            geometry,
        }
    });
    cx.run_until_parked();
    assert_eq!(
        samples.borrow().last().copied(),
        Some(Sample {
            left: Some(0.4),
            right: Some(0.4),
            frame: None
        })
    );
    assert_eq!(
        *geometry.borrow(),
        [
            Bounds::new(point(px(0.0), px(30.0)), size(px(20.0), px(20.0))),
            Bounds::new(point(px(28.0), px(34.0)), size(px(160.0), px(12.0))),
        ]
    );
    phase.set(0.5);
    assert!(frame(&window, cx) > 0);
    assert_eq!(
        samples.borrow().last().copied(),
        Some(Sample {
            left: Some(0.6),
            right: Some(0.6),
            frame: None
        })
    );
    window
        .update(cx, |probe, _, cx| {
            probe.mounted = false;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    let before = samples.borrow().len();
    assert!(frame(&window, cx) <= 1);
    assert_eq!(frame(&window, cx), 0);
    assert_eq!(samples.borrow().len(), before);
    phase.set(0.0);
    window
        .update(cx, |probe, _, cx| {
            probe.mounted = true;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        samples.borrow().last().copied(),
        Some(Sample {
            left: Some(0.4),
            right: Some(0.4),
            frame: None
        })
    );
}
