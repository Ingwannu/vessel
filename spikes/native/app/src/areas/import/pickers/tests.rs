//! Actual old-open/new-open controls across the production dispatcher boundary.
use super::*;
use gpui::{
    FocusHandle, InputEvent as _, KeyDownEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent,
    MouseUpEvent, Pixels, Point, VisualTestContext, point, px, size,
};
use std::cell::RefCell;
use vessel_app_view::{Agent, Fleet, Machine, Reach};
const COMMAND_ROOM: usize = 128;

#[derive(Debug, Clone, Copy)]
enum Replacement {
    Reopen,
    ScopeAba,
    Reconnect,
    Rebind,
    Retired,
}
#[derive(Debug, Clone, Copy)]
enum Input {
    Down,
    Up,
    Escape,
    Row,
    Outside,
}
struct Boundary {
    draws: Cell<Option<usize>>,
    focus: RefCell<Option<FocusHandle>>,
}

fn fleet(scope: &Scope) -> Fleet {
    Fleet {
        read_at: None,
        machines: vec![Machine {
            connection_epoch: Some(scope.context.tag().epoch()),
            node: Some(scope.context.node()),
            name: scope.machine.as_str().to_owned(),
            reach: Reach::Online,
            sessions: Vec::new(),
            agents: vec![Agent {
                harness: scope.harness.clone(),
                name: "Codex".into(),
                unavailable: None,
                not_installed: false,
            }],
            protocol: Some(vessel_proto::PROTOCOL_VERSION),
            version: None,
        }],
    }
}
fn profile(scope: &Scope) -> ProfileInfo {
    ProfileInfo {
        name: "work".parse().unwrap(),
        label: None,
        harness: scope.harness.clone(),
        sets: Vec::new(),
        model: None,
    }
}
fn prepare(shell: &mut Shell, scope: &Scope, window: &mut Window, cx: &mut Context<'_, Shell>) {
    if let vessel_app_view::history::Loading::Pending(ticket) = *shell.areas.import.model.loading()
    {
        let pending = shell.areas.import.model.scope().unwrap().clone();
        assert!(shell.areas.import.model.answer(Reply::Listed {
            scope: pending.clone(),
            ticket,
            result: Ok(Listed {
                entries: Vec::new(),
                next: None,
                node: pending.context.node()
            })
        }));
    }
    shell.areas.import.profile_request = None;
    loaded(shell, scope);
    shell.areas.import.profiles = Ok(vec![profile(scope)]);
    shell.areas.import.dialog.focus(window, cx);
}
fn replace(
    shell: &mut Shell,
    original: &Scope,
    how: Replacement,
    epochs: &mut ConnectionEpochs,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Scope {
    let mut current = original.clone();
    match how {
        Replacement::Reopen => {
            shell.close_import(window, cx);
            shell.open_import(window, cx);
        }
        Replacement::ScopeAba => {
            current.cwd = Some("/other".into());
            loaded(shell, &current);
            current = original.clone();
        }
        Replacement::Reconnect | Replacement::Rebind => {
            let node = if matches!(how, Replacement::Reconnect) {
                original.context.node()
            } else {
                NodeId::random()
            };
            current.context = epochs.admit(original.machine.clone(), node).unwrap();
            shell.history_test_fleet(fleet(&current), cx);
            assert!(
                !shell.areas.import.picker.is_open(),
                "Fleet dispatcher retires before returning"
            );
        }
        Replacement::Retired => {
            shell.history_test_retirement(&original.context, cx);
            assert!(
                !shell.areas.import.picker.is_open(),
                "retirement before Devices can early-return"
            );
        }
    }
    if matches!(how, Replacement::Retired) {
        current.context = epochs
            .admit(original.machine.clone(), original.context.node())
            .unwrap();
        shell.history_test_fleet(fleet(&current), cx);
    }
    prepare(shell, &current, window, cx);
    shell.history_picker(Picker::Account, window, cx);
    assert!(shell.areas.import.picker.is_open());
    if matches!(how, Replacement::Retired) {
        shell.history_test_retirement(&original.context, cx);
        assert!(
            shell.areas.import.picker.is_open(),
            "late old retirement keeps successor"
        );
    }
    current
}
fn key(window: &mut Window, cx: &mut gpui::App, name: &'static str) {
    let _event = window.dispatch_event(
        KeyDownEvent {
            keystroke: Keystroke::parse(name).unwrap(),
            is_held: false,
            prefer_character_input: false,
        }
        .to_platform_input(),
        cx,
    );
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
fn assert_current(
    probe: &Entity<Probe>,
    window: &Window,
    cx: &gpui::App,
    boundary: &Boundary,
    scope: &Scope,
) {
    assert_eq!(
        probe.read(cx).renders.get(),
        boundary.draws.get().unwrap(),
        "old input tree retained"
    );
    probe.read(cx).shell.read_with(cx, |shell, cx| {
        let area = &shell.areas.import;
        assert!(area.open && area.picker.is_open());
        assert_eq!(area.model.scope(), Some(scope));
        assert!(!area.model.importing());
        assert_eq!(window.focused(cx), *boundary.focus.borrow());
    });
}

fn positive(
    probe: &Entity<Probe>,
    cx: &mut VisualTestContext,
    commands: &Rc<RefCell<tokio::sync::mpsc::Receiver<vessel_app_link::Command>>>,
    expected: &Scope,
) {
    // No stale highlight: an unhighlighted current Enter must not select a row.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(commands.borrow_mut().try_recv().is_err());
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    let command = commands
        .borrow_mut()
        .try_recv()
        .expect("current B default choice refreshes");
    let vessel_app_link::Command::History(Ask::List { scope, .. }) = command else {
        panic!("history list")
    };
    assert_eq!(&scope, expected);
    probe.read_with(cx, |probe, cx| {
        probe.shell.read_with(cx, |shell, _| {
            assert!(shell.areas.import.open);
            assert!(!shell.areas.import.picker.is_open());
        });
    });
}
fn run(cx: &mut TestAppContext, how: Replacement, input: Input, compact: bool) {
    let epochs = Rc::new(RefCell::new(ConnectionEpochs::default()));
    let mut original = scope();
    original.context = epochs
        .borrow_mut()
        .admit(original.machine.clone(), original.context.node())
        .unwrap();
    let (link, commands) = Link::detached(COMMAND_ROOM).unwrap();
    let commands = Rc::new(RefCell::new(commands));
    let renders = Rc::new(Cell::new(0));
    let (probe, cx) = cx.add_window_view({
        let original = original.clone();
        move |window, cx| {
            let shell = cx.new(|cx| {
                let mut shell = Shell::with_test_link(window, cx, link);
                shell.history_test_fleet(fleet(&original), cx);
                shell.open_import(window, cx);
                shell.areas.import.profile_request = None;
                prepare(&mut shell, &original, window, cx);
                shell
            });
            cx.observe(&shell, |_, _, cx| cx.notify()).detach();
            Probe { shell, renders }
        }
    });
    cx.simulate_resize(size(px(if compact { 390.0 } else { 1280.0 }), px(900.0)));
    cx.run_until_parked();
    let trigger = cx
        .debug_bounds("history-Account")
        .expect("actual account trigger");
    cx.simulate_click(trigger.center(), Modifiers::default());
    cx.run_until_parked();
    while commands.borrow_mut().try_recv().is_ok() {}
    let boundary = Rc::new(Boundary {
        draws: Cell::new(None),
        focus: RefCell::new(None),
    });
    let current = Rc::new(RefCell::new(original.clone()));
    let at = if matches!(input, Input::Outside) {
        point(px(3.0), px(3.0))
    } else if compact {
        point(px(120.0), px(818.0))
    } else {
        point(
            px(f32::from(trigger.left()).mul_add(1.0, 20.0)),
            px(f32::from(trigger.bottom()).mul_add(1.0, 25.0)),
        )
    };
    let subscription = cx.update(|_, cx| {
        let epochs = Rc::clone(&epochs);
        let (probe, original, commands, boundary, current) = (
            probe.clone(),
            original.clone(),
            Rc::clone(&commands),
            Rc::clone(&boundary),
            Rc::clone(&current),
        );
        cx.intercept_keystrokes(move |_, window, cx| {
            if boundary.draws.get().is_some() {
                return;
            }
            boundary.draws.set(Some(probe.read(cx).renders.get()));
            probe.update(cx, |probe, cx| {
                probe.shell.update(cx, |shell, cx| {
                    *current.borrow_mut() =
                        replace(shell, &original, how, &mut epochs.borrow_mut(), window, cx);
                    *boundary.focus.borrow_mut() = window.focused(cx);
                });
            });
            while commands.borrow_mut().try_recv().is_ok() {}
            if matches!(input, Input::Row | Input::Outside) {
                click(window, cx, at);
            }
        })
    });
    cx.update(|window, cx| {
        key(
            window,
            cx,
            match input {
                Input::Down => "down",
                Input::Up => "up",
                Input::Escape => "escape",
                Input::Row | Input::Outside => "left",
            },
        );
        assert_current(&probe, window, cx, &boundary, &current.borrow());
    });
    assert!(
        commands.borrow_mut().try_recv().is_err(),
        "retained input sends no RPC"
    );
    drop(subscription);
    cx.run_until_parked();
    positive(&probe, cx, &commands, &current.borrow());
}
#[gpui::test]
fn already_open_popup_rejects_old_input_after_reopen_scope_and_connection_changes(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
    for compact in [false, true] {
        for how in [
            Replacement::Reopen,
            Replacement::ScopeAba,
            Replacement::Reconnect,
            Replacement::Rebind,
            Replacement::Retired,
        ] {
            for input in [
                Input::Down,
                Input::Up,
                Input::Escape,
                Input::Row,
                Input::Outside,
            ] {
                run(cx, how, input, compact);
            }
        }
    }
}
