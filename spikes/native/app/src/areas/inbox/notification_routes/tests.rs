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
    // A duplicate has no recipient and cannot pick the newest window.
    first.update(cx, |shell, _| {
        shell.route = Route::Draft;
    });
    click(tag, cx);
    first.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));
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
    successor.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));

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
fn process_routes_are_bounded_and_dead_recipients_have_an_explicit_unavailable_result(
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
            Opened::GenericUnavailable
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
