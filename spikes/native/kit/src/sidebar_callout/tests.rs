//! Literal source geometry and actual pointer dispatch for the shared strip.
use super::*;
use crate::Scheme;
use gpui::{Bounds, Modifiers, Pixels, Render, TestAppContext, point, size};
use std::cell::{Cell, RefCell};

type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
#[derive(Clone, Copy)]
enum Mode {
    Strip,
    Actions,
    Dismiss,
}
struct Probe {
    mode: Mode,
    scheme: Scheme,
    seen: Seen,
    presses: Rc<Cell<usize>>,
}
fn handler(presses: &Rc<Cell<usize>>) -> OnPress<Probe> {
    let presses = Rc::clone(presses);
    Rc::new(move |_, _, _| presses.set(presses.get() + 1))
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let seen = Rc::clone(&self.seen);
        match self.mode {
            Mode::Strip => div()
                .w(px(320.0))
                .flex()
                .flex_col()
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
                .child(sidebar_callout(
                    "notice",
                    Content {
                        title: Some("Setup is missing".into()),
                        description: Some(Description::Text("Configure setup".into())),
                        icon: None,
                        actions: Actions::Two(
                            Action::new("Open", ActionKind::Primary, handler(&self.presses)),
                            Action::new("Later", ActionKind::Secondary, handler(&self.presses))
                                .disabled(),
                        ),
                        dismiss: Some(Dismiss {
                            press: handler(&self.presses),
                        }),
                    },
                    Variant::Default,
                    &tokens,
                    cx,
                ))
                .child(div().h(px(1.0))),
            Mode::Actions => div()
                .w(px(288.0))
                .flex()
                .gap(px(8.0))
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
                .child(action_button(
                    "enabled".into(),
                    Action::new("Open", ActionKind::Primary, handler(&self.presses)),
                    &tokens,
                    cx,
                ))
                .child(action_button(
                    "disabled".into(),
                    Action::new("Later", ActionKind::Secondary, handler(&self.presses)).disabled(),
                    &tokens,
                    cx,
                )),
            Mode::Dismiss => {
                div()
                    .size_full()
                    .child(
                        div()
                            .absolute()
                            .left(px(20.0))
                            .top(px(20.0))
                            .child(dismiss_button(
                                "dismiss".into(),
                                Dismiss {
                                    press: handler(&self.presses),
                                },
                                &tokens,
                                cx,
                            )),
                    )
            }
        }
    }
}
fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
}
#[gpui::test]
fn sidebar_callout_source_strip_and_action_bounds_are_theme_independent(cx: &mut TestAppContext) {
    for scheme in [Scheme::Dark, Scheme::Light] {
        let seen = Seen::default();
        let presses = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            let presses = Rc::clone(&presses);
            move |_, _| Probe {
                mode: Mode::Strip,
                scheme,
                seen,
                presses,
            }
        });
        cx.run_until_parked();
        // 21 title +16 description +34 actions +two8 gaps +8 action margin,
        // with24 vertical padding and one top border; no fixed height budget.
        assert_eq!(
            *seen.borrow(),
            [
                bounds(0.0, 0.0, 320.0, 120.0),
                bounds(0.0, 120.0, 320.0, 1.0)
            ]
        );
    }
}
#[gpui::test]
fn sidebar_callout_private_actions_dispatch_once_and_disabled_never_dispatches(
    cx: &mut TestAppContext,
) {
    for scheme in [Scheme::Dark, Scheme::Light] {
        let seen = Seen::default();
        let presses = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            let presses = Rc::clone(&presses);
            move |_, _| Probe {
                mode: Mode::Actions,
                scheme,
                seen,
                presses,
            }
        });
        cx.run_until_parked();
        assert_eq!(
            *seen.borrow(),
            [
                bounds(0.0, 0.0, 140.0, 34.0),
                bounds(148.0, 0.0, 140.0, 34.0)
            ]
        );
        cx.simulate_click(point(px(70.0), px(17.0)), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(presses.get(), 1);
        cx.simulate_click(point(px(218.0), px(17.0)), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(presses.get(), 1);
    }
}
#[gpui::test]
fn sidebar_callout_dismiss_hit_slop_reaches_outside_the_fourteen_point_icon(
    cx: &mut TestAppContext,
) {
    let presses = Rc::new(Cell::new(0));
    let (_, cx) = cx.add_window_view({
        let presses = Rc::clone(&presses);
        move |_, _| Probe {
            mode: Mode::Dismiss,
            scheme: Scheme::Dark,
            seen: Seen::default(),
            presses,
        }
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(16.0), px(27.0)), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(presses.get(), 1);
    cx.simulate_click(point(px(10.0), px(27.0)), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(presses.get(), 1);
}
#[test]
fn sidebar_callout_error_changes_only_the_top_border_and_success_stays_neutral() {
    for scheme in [Scheme::Dark, Scheme::Light] {
        let tokens = Tokens::of(scheme);
        assert_eq!(border(Variant::Default, &tokens), tokens.neutral.border);
        assert_eq!(border(Variant::Success, &tokens), tokens.neutral.border);
        assert_eq!(border(Variant::Error, &tokens), tokens.neutral.destructive);
        assert_eq!(
            action_paint(ActionKind::Primary, &tokens),
            ActionPaint {
                fill: tokens.neutral.foreground,
                border: tokens.neutral.foreground,
                ink: tokens.neutral.surface0
            }
        );
        assert_eq!(
            action_paint(ActionKind::Secondary, &tokens),
            ActionPaint {
                fill: gpui::transparent_black(),
                border: tokens.neutral.border,
                ink: tokens.neutral.foreground
            }
        );
    }
}
