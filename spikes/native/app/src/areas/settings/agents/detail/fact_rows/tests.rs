//! Separate fact values keep separate scroll state and copy their exact text.
use super::*;
use gpui::{
    Bounds, Context, IntoElement, Modifiers, MouseButton, Pixels, Render, ScrollDelta, ScrollEvent,
    TestAppContext, point,
};
use std::{cell::RefCell, rc::Rc};
type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    width: f32,
    version: String,
    program: String,
    version_bounds: Seen,
    program_bounds: Seen,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(vessel_app_kit::Scheme::Dark);
        let version = Rc::clone(&self.version_bounds);
        let program = Rc::clone(&self.program_bounds);
        div()
            .w(px(self.width))
            .flex()
            .flex_col()
            .child(
                row(
                    &Fact {
                        id: "version",
                        label: "Version",
                        value: Value::Code(&self.version),
                        first: true,
                    },
                    &tokens,
                    window,
                    cx,
                )
                .on_children_prepainted(move |bounds, _, _| *version.borrow_mut() = bounds),
            )
            .child(
                row(
                    &Fact {
                        id: "program",
                        label: "Program",
                        value: Value::Code(&self.program),
                        first: false,
                    },
                    &tokens,
                    window,
                    cx,
                )
                .on_children_prepainted(move |bounds, _, _| *program.borrow_mut() = bounds),
            )
    }
}
#[gpui::test]
fn a_long_program_scrolls_independently_of_the_short_version_and_each_copy_is_exact(
    cx: &mut TestAppContext,
) {
    let version_bounds = Seen::default();
    let program_bounds = Seen::default();
    let version = "1.0".to_owned();
    let program = "long path with spaces/".repeat(40);
    let (_, cx) = cx.add_window_view({
        let version_bounds = Rc::clone(&version_bounds);
        let program_bounds = Rc::clone(&program_bounds);
        let version = version.clone();
        let program = program.clone();
        move |_, _| Probe {
            width: 700.0,
            version,
            program,
            version_bounds,
            program_bounds,
        }
    });
    cx.run_until_parked();
    let version_before = cx.debug_bounds("version-content").unwrap();
    let program_before = cx.debug_bounds("program-content").unwrap();
    let program_control = program_bounds.borrow().get(1).copied().unwrap();
    cx.simulate_event(ScrollEvent {
        position: program_control.center(),
        delta: ScrollDelta::Lines(point(-3.0, 0.0)),
        ..ScrollEvent::default()
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("program-content").unwrap().left() < program_before.left());
    assert_eq!(cx.debug_bounds("version-content").unwrap(), version_before);
    for (id, text) in [("version-copy", version), ("program-copy", program)] {
        let copy = cx.debug_bounds(id).unwrap().center();
        cx.simulate_mouse_move(copy, None::<MouseButton>, Modifiers::default());
        cx.simulate_click(copy, Modifiers::default());
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some(text)
        );
    }
}

#[gpui::test]
fn a_wrapped_compact_program_keeps_copy_inside_the_viewport_and_copies_the_whole_path(
    cx: &mut TestAppContext,
) {
    let program = "long path with spaces/".repeat(40);
    let window = cx.open_window(gpui::size(px(390.0), px(844.0)), {
        let program = program.clone();
        move |_, _| Probe {
            width: 390.0,
            version: "1.0".to_owned(),
            program,
            version_bounds: Seen::default(),
            program_bounds: Seen::default(),
        }
    });
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    let copy = cx.debug_bounds("program-copy").unwrap();
    assert!(copy.left() >= px(0.0));
    // The production row's16-point right inset stays outside the target.
    assert!(copy.right() <= px(374.0), "{copy:?}");
    cx.simulate_click(copy.center(), Modifiers::default());
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some(program)
    );
}
