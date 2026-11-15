//! Dispatch actual frame A callbacks after B opens, before another draw.

use super::*;

fn key(window: &mut Window, cx: &mut App, key: &'static str) {
    let _result = window.dispatch_event(
        KeyDownEvent {
            keystroke: Keystroke::parse(key).unwrap(),
            is_held: false,
            prefer_character_input: false,
        }
        .to_platform_input(),
        cx,
    );
}

fn click(window: &mut Window, cx: &mut App, at: gpui::Point<Pixels>) {
    let _down = window.dispatch_event(
        MouseDownEvent {
            position: at,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    let _up = window.dispatch_event(
        MouseUpEvent {
            position: at,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
        }
        .to_platform_input(),
        cx,
    );
}

fn replace(probe: &mut Probe, window: &mut Window, cx: &mut Context<'_, Probe>) {
    probe.context = 2;
    let _lease = probe.state.open(anchor(), Alignment::End, window, cx);
    cx.notify();
}

fn assert_unmodified(probe: &Probe, before: usize) {
    assert_eq!(probe.renders.get(), before, "must dispatch A before redraw");
    assert!(probe.state.is_open(), "B survives A's dismissal");
    assert_eq!(probe.admissions, 0, "A never reaches domain admission");
    assert_eq!(
        probe.body_events, 0,
        "A cannot invoke arbitrary child input"
    );
}

#[gpui::test]
fn retained_pointer_cannot_focus_old_fields_run_body_or_close_new_surface(cx: &mut TestAppContext) {
    for narrow in [false, true] {
        for target in 0..if narrow { 4 } else { 3 } {
            let (probe, cx) = mount(cx, narrow, false);
            let at = match target {
                0 | 1 => probe.read_with(cx, |probe, _| {
                    probe.body.borrow().get(target + 1).unwrap().center()
                }),
                2 => point(px(2.0), px(2.0)),
                3 => cx.debug_bounds("dialog-close").unwrap().center(),
                _ => unreachable!(),
            };
            cx.update(|window, cx| {
                let before = probe.read(cx).renders.get();
                probe.update(cx, |probe, cx| {
                    replace(probe, window, cx);
                    probe.newer.focus(window, cx);
                });
                click(window, cx, at);
                let probe = probe.read(cx);
                assert_unmodified(probe, before);
                assert!(probe.newer.is_focused(window), "A cannot steal newer focus");
            });
            cx.run_until_parked();
            // The fence belongs to A only; freshly rendered B remains usable.
            let at = probe.read_with(cx, |probe, _| probe.body.borrow().get(2).unwrap().center());
            cx.simulate_click(at, Modifiers::none());
            probe.read_with(cx, |probe, _| assert_eq!(probe.body_events, 1));
        }
    }
}

#[gpui::test]
fn retained_keys_are_fenced_before_arbitrary_child_input_and_dismissal(cx: &mut TestAppContext) {
    for narrow in [false, true] {
        for old_key in ["escape", "f7"] {
            let (probe, cx) = mount(cx, narrow, false);
            probe.update_in(cx, |probe, window, cx| probe.inside.focus(window, cx));
            cx.run_until_parked();
            let boundary = Rc::new(Cell::new(None));
            // GPUI redraws before keys. Its real interceptor runs after that
            // redraw but before frame A's actual key listeners are dispatched.
            let subscription = cx.update(|_, cx| {
                let probe = probe.clone();
                let boundary = Rc::clone(&boundary);
                cx.intercept_keystrokes(move |_, window, cx| {
                    if boundary.get().is_none() {
                        boundary.set(Some(probe.read(cx).renders.get()));
                        probe.update(cx, |probe, cx| replace(probe, window, cx));
                    }
                })
            });
            cx.update(|window, cx| {
                key(window, cx, old_key);
                let probe = probe.read(cx);
                assert_unmodified(probe, boundary.get().expect("real interceptor ran"));
                assert!(probe.state.dialog.focus_handle().is_focused(window));
            });
            drop(subscription);
            cx.run_until_parked();
            probe.update_in(cx, |probe, window, cx| probe.inside.focus(window, cx));
            cx.simulate_keystrokes("f7");
            probe.read_with(cx, |probe, _| assert_eq!(probe.body_events, 1));
        }
    }
}
