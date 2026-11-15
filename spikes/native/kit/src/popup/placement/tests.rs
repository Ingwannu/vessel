use super::*;
use gpui::prelude::*;
use gpui::{Context, Modifiers, MouseButton, Render, TestAppContext, Window, deferred, div, size};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
}
fn options(alignment: Alignment, side: Side) -> Options {
    Options {
        alignment,
        side,
        offset: px(4.0),
        edge: px(8.0),
    }
}
#[test]
fn measured_policy_matches_literal_reference_flip_alignment_and_clamp_cases() {
    let viewport = rect(0.0, 0.0, 400.0, 300.0);
    for (trigger, content, alignment, requested, x, y, actual) in [
        (
            rect(330.0, 260.0, 40.0, 20.0),
            size(px(100.0), px(80.0)),
            Alignment::Start,
            Side::Bottom,
            292.0,
            176.0,
            Side::Top,
        ),
        (
            rect(330.0, 260.0, 40.0, 20.0),
            size(px(100.0), px(80.0)),
            Alignment::End,
            Side::Bottom,
            270.0,
            176.0,
            Side::Top,
        ),
        (
            rect(10.0, 10.0, 30.0, 20.0),
            size(px(120.0), px(100.0)),
            Alignment::Start,
            Side::Top,
            10.0,
            34.0,
            Side::Bottom,
        ),
        (
            rect(10.0, 10.0, 30.0, 20.0),
            size(px(120.0), px(100.0)),
            Alignment::End,
            Side::Top,
            8.0,
            34.0,
            Side::Bottom,
        ),
        (
            rect(100.0, 140.0, 20.0, 20.0),
            size(px(100.0), px(200.0)),
            Alignment::Start,
            Side::Bottom,
            100.0,
            92.0,
            Side::Bottom,
        ),
        (
            rect(100.0, 140.0, 20.0, 20.0),
            size(px(100.0), px(200.0)),
            Alignment::Start,
            Side::Top,
            100.0,
            8.0,
            Side::Top,
        ),
        // Raw room is exactly the content height: offset/edge do not cause a flip.
        (
            rect(100.0, 100.0, 20.0, 20.0),
            size(px(80.0), px(180.0)),
            Alignment::Start,
            Side::Bottom,
            100.0,
            112.0,
            Side::Bottom,
        ),
        (
            rect(330.0, 260.0, 40.0, 20.0),
            size(px(500.0), px(400.0)),
            Alignment::Start,
            Side::Bottom,
            8.0,
            8.0,
            Side::Top,
        ),
        (
            rect(50.5, 60.25, 30.5, 15.5),
            size(px(40.25), px(20.5)),
            Alignment::End,
            Side::Bottom,
            40.75,
            79.75,
            Side::Bottom,
        ),
    ] {
        assert_eq!(
            compute(trigger, content, viewport, options(alignment, requested)),
            Placed {
                origin: point(px(x), px(y)),
                side: actual
            },
        );
    }
}
#[test]
fn measured_policy_uses_the_supplied_nonzero_viewport_origin() {
    assert_eq!(
        compute(
            rect(380.0, 320.0, 40.0, 20.0),
            size(px(100.0), px(80.0)),
            rect(50.0, 60.0, 400.0, 300.0),
            options(Alignment::Start, Side::Bottom)
        ),
        Placed {
            origin: point(px(342.0), px(236.0)),
            side: Side::Top
        },
    );
}

struct Probe {
    trigger: Bounds<Pixels>,
    content: Size<Pixels>,
    options: Options,
    shift: Pixels,
    seen: Rc<RefCell<Option<Bounds<Pixels>>>>,
    presses: Rc<Cell<usize>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let seen = Rc::clone(&self.seen);
        let presses = Rc::clone(&self.presses);
        // The parent inset ensures the wrapper uses window coordinates,
        // rather than accidentally working only at the parent's origin.
        div().size_full().p(px(20.0)).child(deferred(measured(
            self.trigger,
            self.options,
            div()
                .relative()
                .top(self.shift)
                .w(self.content.width)
                .h(self.content.height)
                .on_children_prepainted(move |bounds, _, _| {
                    *seen.borrow_mut() = bounds.first().copied()
                })
                .child(
                    div()
                        .size_full()
                        .on_mouse_down(MouseButton::Left, move |_, _, _| {
                            presses.set(presses.get().saturating_add(1));
                        }),
                ),
        )))
    }
}
#[gpui::test]
fn measured_native_child_flips_with_its_real_size_and_keeps_actual_hit_bounds(
    cx: &mut TestAppContext,
) {
    for (alignment, height, shift, expected) in [
        (Alignment::Start, 80.0, 0.0, rect(292.0, 176.0, 100.0, 80.0)),
        (Alignment::End, 120.0, 0.0, rect(270.0, 136.0, 100.0, 120.0)),
        // The caller's local entrance offset must survive measured placement.
        (Alignment::Start, 80.0, 4.0, rect(292.0, 180.0, 100.0, 80.0)),
    ] {
        let seen = Rc::new(RefCell::new(None));
        let presses = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            let presses = Rc::clone(&presses);
            move |_, _| Probe {
                trigger: rect(330.0, 260.0, 40.0, 20.0),
                content: size(px(100.0), px(height)),
                options: options(alignment, Side::Bottom),
                shift: px(shift),
                seen,
                presses,
            }
        });
        cx.simulate_resize(size(px(400.0), px(300.0)));
        cx.run_until_parked();
        assert_eq!(
            *seen.borrow(),
            Some(expected),
            "actual prepaint child bounds"
        );
        cx.simulate_click(expected.center(), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(
            presses.get(),
            1,
            "translated rectangle receives native input"
        );
        cx.simulate_click(point(px(40.0), px(40.0)), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(presses.get(), 1, "old parent-relative origin is not a hit");
        cx.simulate_click(point(px(350.0), px(280.0)), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(presses.get(), 1, "the unflipped trigger edge is not a hit");
    }
}
