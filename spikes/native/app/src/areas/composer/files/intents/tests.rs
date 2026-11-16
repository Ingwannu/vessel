use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;
use std::sync::Arc;
use vessel_app_view::composer::tray::Origin;

#[gpui::test]
fn retained_pill_callbacks_never_open_or_remove_a_shifted_or_recreated_item(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let target = DraftKey::New;
        let tray = shell.areas.composer.trays.tray(target);
        let first = tray
            .add("a.png", 1, Origin::Pasted(Arc::from([1])))
            .unwrap();
        let _second = tray
            .add("b.png", 1, Origin::Pasted(Arc::from([2])))
            .unwrap();
        let a = tray.items().first().unwrap().clone();
        let (open_a, remove_a) = callbacks(target, &a);
        assert!(tray.remove(first));
        remove_a(shell, 0, window, cx);
        open_a.as_ref().unwrap()(shell, 0, window, cx);
        assert_eq!(
            shell
                .areas
                .composer
                .trays
                .get(target)
                .unwrap()
                .items()
                .len(),
            1
        );
        assert!(!shell.areas.composer.lightbox.is_open());
        // A newly created tray may reuse key zero, even with identical metadata.
        drop(shell.areas.composer.trays.remove(target));
        let tray = shell.areas.composer.trays.tray(target);
        let recreated = tray
            .add("a.png", 1, Origin::Pasted(Arc::from([1])))
            .unwrap();
        assert_eq!(recreated, first);
        remove_a(shell, 0, window, cx);
        open_a.as_ref().unwrap()(shell, 0, window, cx);
        let tray = shell.areas.composer.trays.get(target).unwrap();
        assert_eq!(tray.items().len(), 1);
        assert!(!shell.areas.composer.lightbox.is_open());
        let (_open, remove_current) = callbacks(target, tray.items().first().unwrap());
        remove_current(shell, 0, window, cx);
        assert!(!shell.composer_has_files(target));
    });
}

#[gpui::test]
fn retained_retry_cannot_restart_the_next_failed_upload(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let target = DraftKey::Session {
            session: vessel_proto::SessionId::random(),
        };
        let tray = shell.areas.composer.trays.tray(target);
        let a = tray
            .add("a.txt", 1, Origin::Pasted(Arc::from([1])))
            .unwrap();
        let b = tray
            .add("b.txt", 1, Origin::Pasted(Arc::from([2])))
            .unwrap();
        tray.failed(a, Problem::Changed, Again::Restart);
        tray.failed(b, Problem::Changed, Again::Restart);
        let (retry_a, _remove) = callbacks(target, tray.items().first().unwrap());
        assert!(tray.remove(a));
        retry_a.unwrap()(shell, 0, window, cx);
        assert!(matches!(
            shell
                .areas
                .composer
                .trays
                .get(target)
                .unwrap()
                .items()
                .first()
                .unwrap()
                .state,
            ItemState::Failed {
                again: Again::Restart,
                ..
            }
        ));
    });
}
