//! Actual Shell roots exercise original-window routing and deferred refusal.

use super::*;
use crate::host_windows::register;
use gpui::{AnyWindowHandle, AppContext as _, Entity, FocusHandle, TestAppContext, point, px};
use std::cell::RefCell;
use vessel_app_kit::field::context_menu::{CopyPermission, Editing, Facts, PastePermission};

fn make(cx: &mut TestAppContext, wrapped: bool) -> (AnyWindowHandle, Entity<Shell>, FocusHandle) {
    let kept = Rc::new(RefCell::new(None));
    let window = cx.update(|cx| {
        if wrapped {
            cx.open_window(gpui::WindowOptions::default(), |window, cx| {
                let shell = mount(window, cx, &kept);
                vessel_app_kit::root(shell, window, cx)
            })
            .unwrap()
            .into()
        } else {
            cx.open_window(gpui::WindowOptions::default(), |window, cx| {
                mount(window, cx, &kept)
            })
            .unwrap()
            .into()
        }
    });
    let (shell, focus) = kept.borrow_mut().take().unwrap();
    (window, shell, focus)
}

fn mount(
    window: &mut Window,
    cx: &mut App,
    kept: &RefCell<Option<(Entity<Shell>, FocusHandle)>>,
) -> Entity<Shell> {
    let shell = cx.new(|cx| Shell::without_io(window, cx));
    let focus = cx.focus_handle();
    focus.focus(window, cx);
    register(&shell, window, cx);
    *kept.borrow_mut() = Some((shell.clone(), focus));
    shell
}

fn request(focus: &FocusHandle) -> Request {
    Request::new(
        point(px(20.0), px(16.0)),
        focus.downgrade(),
        Rc::new(|_| {
            Some(Facts {
                copy: CopyPermission::Available,
                paste: PastePermission::Available,
                editing: Editing::Editable,
            })
        }),
    )
}

fn callbacks() -> Callbacks {
    Callbacks {
        admitted: |shell| shell.notice.is_none(),
        open: |shell, request, window, cx| {
            assert!(request.current(window, cx));
            shell.notice = Some("context delivered".to_owned());
            true
        },
    }
}

#[gpui::test]
fn original_direct_and_wrapped_roots_receive_the_deferred_request_once(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    for wrapped in [false, true] {
        let (window, shell, focus) = make(cx, wrapped);
        cx.update(|cx| {
            window
                .update(cx, |_, window, cx| {
                    handler(callbacks())(request(&focus), window, cx);
                })
                .unwrap();
        });
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(shell.read(cx).notice.as_deref(), Some("context delivered"));
            window
                .update(cx, |_, window, _| window.remove_window())
                .unwrap();
        });
        cx.run_until_parked();
    }
}

#[derive(Clone, Copy)]
enum Retire {
    Root,
    Record,
    Focus,
    Admission,
    Closed,
}

#[gpui::test]
fn deferred_request_cannot_retarget_replaced_closed_or_refused_recipients(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    for retired in [
        Retire::Root,
        Retire::Record,
        Retire::Focus,
        Retire::Admission,
        Retire::Closed,
    ] {
        let (window, original, focus) = make(cx, false);
        let (_other_window, other, _other_focus) = make(cx, false);
        cx.update(|cx| {
            window
                .update(cx, |_, window, cx| {
                    focus.focus(window, cx);
                    handler(callbacks())(request(&focus), window, cx);
                    match retired {
                        Retire::Root => {
                            let next = window.replace_root(cx, Shell::without_io);
                            register(&next, window, cx);
                        }
                        Retire::Record => cx.global_mut::<Registry>().records.clear(),
                        Retire::Focus => {
                            cx.focus_handle().focus(window, cx);
                        }
                        Retire::Admission => original.update(cx, |shell, _| {
                            shell.notice = Some("blocked".to_owned());
                        }),
                        Retire::Closed => window.remove_window(),
                    }
                })
                .unwrap();
        });
        cx.run_until_parked();
        cx.update(|cx| {
            let expected = if matches!(retired, Retire::Admission) {
                Some("blocked")
            } else {
                None
            };
            assert_eq!(original.read(cx).notice.as_deref(), expected);
            assert!(
                other.read(cx).notice.is_none(),
                "there is no focused-window fallback"
            );
            for live in cx.windows() {
                live.update(cx, |_, window, _| window.remove_window())
                    .unwrap();
            }
        });
        cx.run_until_parked();
    }
}
