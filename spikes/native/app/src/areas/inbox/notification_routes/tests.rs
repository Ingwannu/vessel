//! Exercise the registered platform callback with real windows and kit roots.
use super::*;
use gpui::{AppContext as _, Entity, SystemNotificationResponse, TestAppContext, WindowOptions};
use vessel_app_view::{
    connection::{ConnectionEpochs, MachineKey},
    inbox::terminals::Key,
};
use vessel_proto::{NodeId, TerminalId};

fn window(cx: &mut TestAppContext) -> (Entity<Shell>, AnyWindowHandle) {
    cx.update(|cx| {
        let mut shell = None;
        let window = cx
            .open_window(WindowOptions::default(), |window, cx| {
                let content = cx.new(|cx| Shell::without_io(window, cx));
                crate::host_windows::register(&content, window, cx);
                shell = Some(content.clone());
                vessel_app_kit::root(content, window, cx)
            })
            .unwrap();
        (shell.unwrap(), window.into())
    })
}
fn source() -> Source {
    Source::new(
        ConnectionEpochs::default()
            .admit(MachineKey::from_book("first"), NodeId::random())
            .unwrap(),
    )
    .unwrap()
}
fn notice(
    shell: &Entity<Shell>,
    handle: AnyWindowHandle,
    source: &Source,
    cx: &mut TestAppContext,
) -> (DeliveryTag, Key) {
    let tag = DeliveryTag::new(uuid::Uuid::new_v4().into_bytes());
    let key = Key {
        node: source.node(),
        terminal: TerminalId::random(),
    };
    handle
        .update(cx, |_, window, cx| {
            shell.update(cx, |shell, cx| {
                shell.areas.inbox.targets = vec![Target::Online(source.clone())];
                remember(tag, source.clone(), Destination::Terminal(key), window, cx);
            });
        })
        .unwrap();
    (tag, key)
}
fn click(tag: DeliveryTag, cx: &TestAppContext) {
    cx.simulate_system_notification_response(SystemNotificationResponse {
        tag: super::super::notifications::tag(tag).into(),
        action_id: None,
    });
}

#[gpui::test]
fn later_window_does_not_replace_the_process_callback_or_steal_an_earlier_notice(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (first, first_window) = window(cx);
    let (tag, key) = notice(&first, first_window, &source(), cx);
    let (second, second_window) = window(cx);
    let (second_tag, second_key) = notice(&second, second_window, &source(), cx);
    click(tag, cx);
    first.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Inbox);
        assert_eq!(shell.areas.inbox.terminals.selected, Some(key));
    });
    second.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));
    click(second_tag, cx);
    second.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Inbox);
        assert_eq!(shell.areas.inbox.terminals.selected, Some(second_key));
    });
    // A duplicate has no source authority; the current host gets generic Inbox.
    first.update(cx, |shell, _| {
        shell.route = Route::Draft;
    });
    second.update(cx, |shell, _| {
        shell.route = Route::Draft;
        shell.areas.inbox.terminals.selected = None;
    });
    second_window
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    click(tag, cx);
    first.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));
    second.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Inbox);
        assert_eq!(shell.areas.inbox.terminals.selected, None);
    });
}

#[gpui::test]
fn closed_window_and_replaced_root_cannot_route_to_a_retained_or_successor_shell(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (old, old_window) = window(cx);
    let (old_tag, _) = notice(&old, old_window, &source(), cx);
    old_window
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    let (successor, successor_window) = window(cx);
    assert_ne!(old_window, successor_window);
    click(old_tag, cx);
    old.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));
    successor.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Inbox);
        assert_eq!(shell.areas.inbox.terminals.selected, None);
    });
    successor.update(cx, |shell, _| {
        shell.route = Route::Draft;
    });

    let (tag, _) = notice(&successor, successor_window, &source(), cx);
    let replacement = successor_window
        .update(cx, |_, window, cx| {
            window.replace_root(cx, Shell::without_io)
        })
        .unwrap();
    click(tag, cx);
    successor.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));
    replacement.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));
}

#[gpui::test]
fn exact_connection_retirement_only_opens_the_original_inbox_without_source_navigation(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let mut epochs = ConnectionEpochs::default();
    let machine = MachineKey::from_book("same-alias");
    let node = NodeId::random();
    let old = Source::new(epochs.admit(machine.clone(), node).unwrap()).unwrap();
    let new = Source::new(epochs.admit(machine, node).unwrap()).unwrap();
    let (shell, handle) = window(cx);
    let (tag, _) = notice(&shell, handle, &old, cx);
    shell.update(cx, |shell, _| {
        shell.areas.inbox.targets = vec![Target::Online(new)];
    });
    click(tag, cx);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Inbox);
        assert_eq!(shell.areas.inbox.terminals.selected, None);
    });
}

#[gpui::test]
fn process_routes_are_bounded_and_missing_hosts_have_an_explicit_unavailable_result(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (shell, handle) = window(cx);
    let source = source();
    let (oldest, _) = notice(&shell, handle, &source, cx);
    for _ in 0..MAX_PENDING_NOTICES {
        let _notice = notice(&shell, handle, &source, cx);
    }
    cx.update(|cx| {
        assert_eq!(cx.global::<Routes>().entries.len(), MAX_PENDING_NOTICES);
        assert_eq!(
            open(&super::super::notifications::tag(oldest), cx),
            Opened::Generic
        );
        assert_eq!(open("another-feature", cx), Opened::OtherNotification);
    });
    let (last, _) = notice(&shell, handle, &source, cx);
    handle
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    drop(shell);
    cx.update(|cx| {
        assert_eq!(
            open(&super::super::notifications::tag(last), cx),
            Opened::GenericUnavailable
        );
    });
}

#[gpui::test]
fn registered_replacement_receives_generic_inbox_without_inheriting_the_old_destination(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (old, handle) = window(cx);
    let original_source = source();
    let (tag, _) = notice(&old, handle, &original_source, cx);
    let replacement = handle
        .update(cx, |_, window, cx| {
            let replacement = window.replace_root(cx, Shell::without_io);
            crate::host_windows::register(&replacement, window, cx);
            replacement
        })
        .unwrap();
    // Even an otherwise matching connection does not transfer a notice's target.
    replacement.update(cx, |shell, _| {
        shell.areas.inbox.targets = vec![Target::Online(original_source)];
    });
    click(tag, cx);
    old.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));
    replacement.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Inbox);
        assert_eq!(shell.areas.inbox.terminals.selected, None);
    });
}

fn factory(cx: &mut App) -> crate::host_windows::Delivery {
    match cx.open_window(WindowOptions::default(), |window, cx| {
        let shell = cx.new(|cx| Shell::without_io(window, cx));
        crate::host_windows::register(&shell, window, cx);
        vessel_app_kit::root(shell, window, cx)
    }) {
        Ok(_) => crate::host_windows::Delivery::Delivered,
        Err(_) => crate::host_windows::Delivery::Unavailable,
    }
}

#[gpui::test]
fn callback_survives_all_windows_closing_and_uses_only_the_desktop_factory(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (old, handle) = window(cx);
    let (tag, _) = notice(&old, handle, &source(), cx);
    handle
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(cx.windows().is_empty());
        crate::host_windows::set_factory(factory, cx);
    });
    click(tag, cx);
    cx.update(|cx| {
        assert_eq!(cx.windows().len(), 1);
        let records = &cx.global::<crate::host_windows::Registry>().records;
        assert_eq!(records.len(), 1);
        let shell = records[0].shell.upgrade().unwrap();
        assert_ne!(shell.entity_id(), old.entity_id());
        assert_eq!(shell.read(cx).route, Route::Inbox);
        assert_eq!(shell.read(cx).areas.inbox.terminals.selected, None);
    });
    old.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));
}

#[gpui::test]
fn deferred_notice_update_does_not_consume_work_in_a_retained_replaced_shell(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (old, handle) = window(cx);
    let original_source = source();
    let pending = old.update(cx, |shell, _cx| {
        vessel_app_view::inbox::admission::Pending {
            key: vessel_app_view::inbox::Key {
                node: original_source.node(),
                session: vessel_proto::SessionId::random(),
                seq: vessel_proto::EventSeq::new(1),
            },
            position: vessel_app_view::inbox::Position::new(1),
            request: shell.areas.inbox.store.ticket().unwrap(),
            source: original_source,
        }
    });
    old.update(cx, |shell, cx| {
        shell.areas.inbox.granted.push(pending.clone());
        shell.inbox_notice_update(cx);
    });
    cx.run_until_parked();
    old.read_with(cx, |shell, _| assert!(shell.areas.inbox.granted.is_empty()));
    old.update(cx, |shell, cx| {
        shell.areas.inbox.granted.push(pending);
        shell.inbox_notice_update(cx);
    });
    // Replace before the deferred task can enter the window; retain the old
    // entity deliberately, as a retained pane/owner may do during teardown.
    let replacement = handle
        .update(cx, |_, window, cx| {
            window.replace_root(cx, Shell::without_io)
        })
        .unwrap();
    cx.run_until_parked();
    old.read_with(cx, |shell, _| {
        assert_eq!(shell.areas.inbox.granted.len(), 1);
    });
    replacement.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Draft);
        assert!(shell.areas.inbox.granted.is_empty());
    });
}
