//! Retain actual rendered controls across replacement in a real dispatcher.
use super::*;
use gpui::{
    FocusHandle, InputEvent as _, KeyDownEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent,
    MouseUpEvent, Pixels, Point, Subscription, VisualTestContext, px, size,
};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug)]
enum Path {
    Row,
    Import,
    Glyph,
    Escape,
}
#[derive(Clone, Copy)]
struct Case {
    aba: bool,
    path: Path,
}
#[derive(Default)]
struct Boundary {
    frame: Cell<Option<usize>>,
    focus: RefCell<Option<FocusHandle>>,
}

fn replace(
    shell: &mut Shell,
    original: &Scope,
    aba: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if !aba {
        shell.close_import(window, cx);
        shell.open_import(window, cx);
    }
    if aba {
        let mut other = original.clone();
        other.cwd = Some("/other".into());
        loaded(shell, &other);
    }
    loaded(shell, original);
    shell.areas.import.dialog.focus(window, cx);
}

fn click(window: &mut Window, cx: &mut gpui::App, at: Point<Pixels>) {
    let _down = window.dispatch_event(
        MouseDownEvent {
            position: at,
            button: MouseButton::Left,
            modifiers: Modifiers::default(),
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
            modifiers: Modifiers::default(),
            click_count: 1,
        }
        .to_platform_input(),
        cx,
    );
}

fn intercept(
    cx: &mut VisualTestContext,
    probe: &Entity<Probe>,
    original: &Scope,
    case: Case,
    at: Point<Pixels>,
) -> (Rc<Boundary>, Subscription) {
    let boundary = Rc::new(Boundary::default());
    let subscription = cx.update(|_, cx| {
        let probe = probe.clone();
        let original = original.clone();
        let boundary = Rc::clone(&boundary);
        cx.intercept_keystrokes(move |_, window, cx| {
            if boundary.frame.get().is_some() {
                return;
            }
            boundary.frame.set(Some(probe.read(cx).renders.get()));
            probe.update(cx, |probe, cx| {
                probe.shell.update(cx, |shell, cx| {
                    replace(shell, &original, case.aba, window, cx);
                    *boundary.focus.borrow_mut() = window.focused(cx);
                });
            });
            if !matches!(case.path, Path::Escape) {
                click(window, cx, at);
            }
        })
    });
    (boundary, subscription)
}

fn old_input(cx: &mut VisualTestContext, probe: &Entity<Probe>, boundary: &Boundary, case: Case) {
    cx.update(|window, cx| {
        let _event = window.dispatch_event(
            KeyDownEvent {
                keystroke: Keystroke::parse(if matches!(case.path, Path::Escape) {
                    "escape"
                } else {
                    "left"
                })
                .unwrap(),
                is_held: false,
                prefer_character_input: false,
            }
            .to_platform_input(),
            cx,
        );
        assert_eq!(
            probe.read(cx).renders.get(),
            boundary.frame.get().unwrap(),
            "retained production input tree"
        );
        probe.read(cx).shell.read_with(cx, |shell, cx| {
            assert!(shell.areas.import.open, "{:?}/{}", case.path, case.aba);
            assert!(shell.areas.import.model.has_selection());
            assert!(!shell.areas.import.model.importing());
            assert_eq!(window.focused(cx), *boundary.focus.borrow());
        });
    });
}

fn current_input(
    cx: &mut VisualTestContext,
    probe: &Entity<Probe>,
    commands: &mut tokio::sync::mpsc::Receiver<vessel_app_link::Command>,
) {
    let row = cx
        .debug_bounds("history-row-native")
        .expect("production history row")
        .center();
    cx.simulate_click(row, Modifiers::default());
    probe.read_with(cx, |probe, cx| {
        probe.shell.read_with(cx, |shell, _| {
            assert!(!shell.areas.import.model.has_selection());
        });
    });
    let row = cx
        .debug_bounds("history-row-native")
        .expect("current production history row")
        .center();
    cx.simulate_click(row, Modifiers::default());
    probe.read_with(cx, |probe, cx| {
        probe.shell.read_with(cx, |shell, _| {
            assert!(shell.areas.import.model.has_selection());
        });
    });
    let submit = cx.debug_bounds("history-submit").unwrap().center();
    cx.simulate_click(submit, Modifiers::default());
    assert!(matches!(
        commands.try_recv().unwrap(),
        vessel_app_link::Command::History(Ask::Import(_))
    ));
    let close = cx.debug_bounds("dialog-close").unwrap().center();
    cx.simulate_click(close, Modifiers::default());
    probe.read_with(cx, |probe, cx| {
        probe.shell.read_with(cx, |shell, _| {
            assert!(!shell.areas.import.open);
        });
    });
}

fn run_case(cx: &mut TestAppContext, case: Case) {
    let original = scope();
    let renders = Rc::new(Cell::new(0));
    let (link, mut commands) = Link::detached(8).unwrap();
    let (probe, cx) = cx.add_window_view({
        let original = original.clone();
        move |window, cx| {
            let shell = cx.new(|cx| {
                let mut shell = Shell::with_test_link(window, cx, link);
                shell.open_import(window, cx);
                loaded(&mut shell, &original);
                shell
            });
            // The fixture renders the child's controls directly, so its root
            // must observe child notifications to rebuild captured intentions.
            cx.observe(&shell, |_, _, cx| cx.notify()).detach();
            Probe { shell, renders }
        }
    });
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    cx.run_until_parked();
    let at = match case.path {
        Path::Row => cx
            .debug_bounds("history-row-native")
            .expect("rendered actual row")
            .center(),
        Path::Import => cx.debug_bounds("history-submit").unwrap().center(),
        Path::Glyph | Path::Escape => cx.debug_bounds("dialog-close").unwrap().center(),
    };
    let (boundary, subscription) = intercept(cx, &probe, &original, case, at);
    old_input(cx, &probe, &boundary, case);
    drop(subscription);
    assert!(commands.try_recv().is_err(), "stale intentions send no RPC");
    cx.run_until_parked();
    current_input(cx, &probe, &mut commands);
}

#[gpui::test]
fn retained_callbacks_refuse_reopen_and_scope_aba_before_any_effect(cx: &mut TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
    for aba in [false, true] {
        for path in [Path::Row, Path::Import, Path::Glyph, Path::Escape] {
            run_case(cx, Case { aba, path });
        }
    }
}
