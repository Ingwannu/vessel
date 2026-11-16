use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;

#[gpui::test]
fn retained_context_callbacks_refuse_shifted_replaced_and_recreated_entries(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let target = DraftKey::New;
        let a = super::super::super::prepared::tests::history("first").into_attachment();
        let b = super::super::super::prepared::tests::history("replacement").into_attachment();
        let tray = shell.areas.composer.contexts.tray(target);
        tray.add(a.clone()).unwrap();
        let (intent, item) = tray.visible_intents().next().unwrap();
        let (open_a, remove_a) = callbacks(target, item, intent);
        tray.add(b.clone()).unwrap();
        remove_a(shell, 0, window, cx);
        open_a.as_ref().unwrap()(shell, 0, window, cx);
        assert!(shell.areas.composer.context_preview.item.is_none());
        assert_eq!(
            shell
                .areas
                .composer
                .contexts
                .get(target)
                .unwrap()
                .visible()
                .next(),
            Some(&b)
        );
        drop(shell.areas.composer.contexts.remove(target));
        shell
            .areas
            .composer
            .contexts
            .tray(target)
            .add(a.clone())
            .unwrap();
        remove_a(shell, 0, window, cx);
        open_a.as_ref().unwrap()(shell, 0, window, cx);
        assert!(shell.areas.composer.context_preview.item.is_none());
        let tray = shell.areas.composer.contexts.get(target).unwrap();
        let (intent, item) = tray.visible_intents().next().unwrap();
        let (open_current, remove_current) = callbacks(target, item, intent);
        open_current.unwrap()(shell, 0, window, cx);
        assert_eq!(shell.areas.composer.context_preview.item.as_ref(), Some(&a));
        remove_current(shell, 0, window, cx);
        assert!(!shell.composer_has_context(target));
    });
}
