//! Actual loading-row pitch through the area composition and shared kit,
//! at the reference page's 720-point cap and 390-point compact width.
use super::*;
use gpui::{Bounds, IntoElement, Pixels, Render, TestAppContext, Window, point, size};
use std::{cell::RefCell, rc::Rc};
struct Probe {
    width: f32,
    seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let seen = Rc::clone(&self.seen);
        div()
            .w(px(self.width))
            .max_w(px(720.0))
            .p(px(spacing::S4))
            .child(vessel_app_kit::skeleton::pulse(
                "loading-probe",
                move |opacity| {
                    let seen = Rc::clone(&seen);
                    loading_card(opacity, &Tokens::of(vessel_app_kit::Scheme::Dark))
                        .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
                },
            ))
    }
}
#[gpui::test]
fn four_loading_rows_keep_the_reference_pitch_at_both_page_widths(cx: &mut TestAppContext) {
    for (width, inner) in [(1280.0, 686.0), (390.0, 356.0)] {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let _window = cx.open_window(size(px(width), px(900.0)), {
            let seen = Rc::clone(&seen);
            move |_, _| Probe { width, seen }
        });
        cx.run_until_parked();
        assert_eq!(
            *seen.borrow(),
            [
                Bounds::new(point(px(17.0), px(17.0)), size(px(inner), px(56.0))),
                Bounds::new(point(px(17.0), px(73.0)), size(px(inner), px(56.0))),
                Bounds::new(point(px(17.0), px(129.0)), size(px(inner), px(56.0))),
                Bounds::new(point(px(17.0), px(185.0)), size(px(inner), px(56.0))),
            ]
        );
    }
}

// Exercise the production pulse and four-row card through GPUI's App flag.
// This does not simulate the separate, currently absent OS preference bridge.
type StaticSamples = Rc<RefCell<Vec<(Option<f32>, Option<f32>)>>>;
struct ReducedProbe {
    samples: StaticSamples,
}
impl Render for ReducedProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let samples = Rc::clone(&self.samples);
        let tokens = Tokens::of(vessel_app_kit::Scheme::Dark);
        vessel_app_kit::skeleton::pulse("actual-reduced-loading", move |opacity| {
            let mut shape = vessel_app_kit::skeleton::shape(
                vessel_app_kit::skeleton::Shape {
                    size: size(px(20.0), px(20.0)),
                    rounding: vessel_app_kit::skeleton::Rounding::Medium,
                },
                opacity,
                &tokens,
            );
            let mut card = loading_card(opacity, &tokens);
            samples
                .borrow_mut()
                .push((shape.style().opacity, card.style().opacity));
            card
        })
    }
}
#[gpui::test]
fn initial_loading_with_the_app_reduce_motion_flag_is_static_and_schedules_no_frames(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let samples = StaticSamples::default();
    let window = cx.open_window(size(px(390.0), px(844.0)), {
        let samples = Rc::clone(&samples);
        move |_, _| ReducedProbe { samples }
    });
    cx.run_until_parked();
    assert_eq!(*samples.borrow(), [(Some(0.4), None)]);
    for _ in 0..2 {
        assert_eq!(
            window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap(),
            0
        );
        cx.run_until_parked();
        assert_eq!(*samples.borrow(), [(Some(0.4), None)]);
    }
}
