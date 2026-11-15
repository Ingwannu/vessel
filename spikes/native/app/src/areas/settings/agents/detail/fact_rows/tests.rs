//! Fact values share one trailing edge, a path is cut to its set width,
//! and each copy button copies the exact, whole text.
use super::*;
use gpui::{Bounds, Context, IntoElement, Modifiers, Pixels, Render, TestAppContext};
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
                    None,
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
                        value: Value::Path(&self.program),
                        first: false,
                    },
                    None,
                    &tokens,
                    window,
                    cx,
                )
                .on_children_prepainted(move |bounds, _, _| *program.borrow_mut() = bounds),
            )
    }
}
#[gpui::test]
fn a_long_path_is_cut_to_its_width_ends_with_the_version_and_copies_whole(cx: &mut TestAppContext) {
    let version = "1.0".to_owned();
    let program = "long path with spaces/".repeat(40);
    let (_, cx) = cx.add_window_view({
        let version = version.clone();
        let program = program.clone();
        move |_, _| Probe {
            width: 700.0,
            version,
            program,
            version_bounds: Seen::default(),
            program_bounds: Seen::default(),
        }
    });
    cx.run_until_parked();
    let path = cx.debug_bounds("program-content").unwrap();
    assert!(path.size.width <= px(CONTROL), "{path:?}");
    // One trailing edge: both copy buttons are drawn, side by side in x.
    let version_copy = cx.debug_bounds("version-copy").unwrap();
    let program_copy = cx.debug_bounds("program-copy").unwrap();
    assert_eq!(version_copy.right(), program_copy.right());
    for (copy, text) in [(version_copy, version), (program_copy, program)] {
        cx.simulate_click(copy.center(), Modifiers::default());
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
    // The production row's 16-point right inset stays outside the target.
    assert!(copy.right() <= px(374.0), "{copy:?}");
    cx.simulate_click(copy.center(), Modifiers::default());
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some(program)
    );
}
