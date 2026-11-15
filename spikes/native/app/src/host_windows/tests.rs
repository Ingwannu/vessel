//! Real window/root identity exercises the shared host boundary without nodes.

use super::*;
use gpui::{AppContext as _, Context, IntoElement, Render, TestAppContext, WindowVisibility, div};
use std::cell::RefCell;
use std::rc::Rc;

fn direct(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<Shell>) {
    let window = cx.add_window(|window, cx| {
        let shell = Shell::without_io(window, cx);
        register(&cx.entity(), window, cx);
        shell
    });
    let shell = cx.update(|cx| window.entity(cx).unwrap());
    (window.into(), shell)
}

fn wrapped(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<Shell>) {
    let captured = Rc::new(RefCell::new(None));
    let window = cx.update(|cx| {
        cx.open_window(gpui::WindowOptions::default(), |window, cx| {
            let shell = cx.new(|cx| Shell::without_io(window, cx));
            register(&shell, window, cx);
            *captured.borrow_mut() = Some(shell.clone());
            vessel_app_kit::root(shell, window, cx)
        })
        .unwrap()
    });
    let shell = captured.borrow_mut().take().unwrap();
    (window.into(), shell)
}

fn mark(cx: &mut App) -> Delivery {
    with_focused_shell(cx, |shell, _, _| {
        shell.notice = Some("generic inbox".to_owned());
    })
}

#[gpui::test]
fn direct_and_wrapped_roots_are_routed_once_without_retaining_duplicate_records(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    for make in [direct, wrapped] {
        let (window, shell) = make(cx);
        cx.update(|cx| {
            window
                .update(cx, |_, window, cx| {
                    register(&shell, window, cx);
                    register(&shell, window, cx);
                    window.activate_window();
                })
                .unwrap();
            assert_eq!(cx.global::<Registry>().records.len(), 1);
            assert_eq!(mark(cx), Delivery::Delivered);
            assert_eq!(shell.read(cx).notice.as_deref(), Some("generic inbox"));
            window
                .update(cx, |_, window, _| window.remove_window())
                .unwrap();
        });
        cx.run_until_parked();
        cx.update(|cx| assert!(cx.global::<Registry>().records.is_empty()));
    }
}

struct Empty;
impl Render for Empty {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

#[gpui::test]
fn retained_old_shell_cannot_receive_after_root_replacement_then_new_registration_wins(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (window, old) = direct(cx);
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                let _replacement = window.replace_root(cx, |_, _| Empty);
            })
            .unwrap();
        assert!(
            old.downgrade().upgrade().is_some(),
            "old Shell is deliberately retained"
        );
        assert_eq!(mark(cx), Delivery::Unavailable);
        assert!(old.read(cx).notice.is_none());
        let replacement = window
            .update(cx, |_, window, cx| {
                let replacement = window.replace_root(cx, Shell::without_io);
                register(&replacement, window, cx);
                replacement
            })
            .unwrap();
        assert_eq!(cx.global::<Registry>().records.len(), 1);
        assert_eq!(mark(cx), Delivery::Delivered);
        assert!(old.read(cx).notice.is_none());
        assert_eq!(
            replacement.read(cx).notice.as_deref(),
            Some("generic inbox")
        );
    });
}

#[gpui::test]
fn closed_handle_and_dead_shell_do_not_rebind_to_a_successor_window(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (original, shell) = wrapped(cx);
    let weak = shell.downgrade();
    cx.update(|cx| {
        original
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
    });
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(mark(cx), Delivery::Unavailable);
        assert!(shell.read(cx).notice.is_none());
    });
    drop(shell);
    cx.run_until_parked();
    assert!(weak.upgrade().is_none(), "registry owns no strong Shell");
    let (successor, next) = direct(cx);
    assert_ne!(original, successor);
    cx.update(|cx| {
        assert!(original.update(cx, |_, _, _| {}).is_err());
        assert_eq!(cx.global::<Registry>().records.len(), 1);
        assert_eq!(mark(cx), Delivery::Delivered);
        assert_eq!(next.read(cx).notice.as_deref(), Some("generic inbox"));
    });
}

#[gpui::test]
fn active_then_visible_then_hidden_live_fallback_uses_only_current_root_records(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (first_window, first) = direct(cx);
    let (second_window, second) = wrapped(cx);
    cx.update(|cx| {
        second_window
            .update(cx, |_, window, _| window.activate_window())
            .unwrap();
        assert_eq!(cx.active_window(), Some(second_window));
        assert_eq!(mark(cx), Delivery::Delivered);
        assert!(first.read(cx).notice.is_none());
        second.update(cx, |shell, _| shell.notice = None);
    });
    let unrelated = cx.add_window(|_, _| Empty);
    cx.update(|cx| {
        unrelated
            .update(cx, |_, window, _| window.activate_window())
            .unwrap();
    });
    cx.simulate_window_visibility_change(first_window, WindowVisibility::Hidden);
    cx.update(|cx| {
        assert_eq!(mark(cx), Delivery::Delivered);
        assert!(first.read(cx).notice.is_none());
        assert_eq!(second.read(cx).notice.as_deref(), Some("generic inbox"));
        second.update(cx, |shell, _| shell.notice = None);
    });
    cx.simulate_window_visibility_change(second_window, WindowVisibility::Hidden);
    cx.update(|cx| {
        assert_eq!(mark(cx), Delivery::Delivered);
        assert_eq!(first.read(cx).notice.as_deref(), Some("generic inbox"));
        assert!(second.read(cx).notice.is_none());
    });
}

fn factory(cx: &mut App) -> Delivery {
    match cx.open_window(gpui::WindowOptions::default(), |window, cx| {
        let shell = cx.new(|cx| Shell::without_io(window, cx));
        register(&shell, window, cx);
        shell
    }) {
        Ok(_window) => Delivery::Delivered,
        Err(_error) => Delivery::Unavailable,
    }
}

#[gpui::test]
fn absent_factory_refuses_explicitly_and_installed_factory_creates_a_real_window(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    cx.update(|cx| {
        assert_eq!(open_additional(cx), Delivery::Unavailable);
        assert!(cx.windows().is_empty());
        set_factory(factory, cx);
        assert_eq!(open_additional(cx), Delivery::Delivered);
        assert_eq!(cx.windows().len(), 1);
        assert_eq!(cx.global::<Registry>().records.len(), 1);
        assert_eq!(mark(cx), Delivery::Delivered);
    });
}

mod enumeration;
