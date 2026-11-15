//! Global admission refuses unknown roots instead of silently omitting them.

use super::*;

#[gpui::test]
fn enumeration_covers_all_roots_and_refuses_unregistered_or_replaced_content(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (direct_window, old) = direct(cx);
    let (_wrapped_window, _wrapped) = wrapped(cx);
    cx.update(|cx| {
        assert_eq!(shells(cx).unwrap().len(), 2);
        let unknown = cx
            .open_window(gpui::WindowOptions::default(), |_, cx| cx.new(|_| Empty))
            .unwrap();
        assert_eq!(
            shells(cx).err(),
            Some(Unavailable),
            "unknown roots cannot be omitted"
        );
        unknown
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
        assert_eq!(shells(cx).unwrap().len(), 2);
        let replacement = direct_window
            .update(cx, |_, window, cx| {
                window.replace_root(cx, Shell::without_io)
            })
            .unwrap();
        assert!(old.downgrade().upgrade().is_some());
        assert_eq!(
            shells(cx).err(),
            Some(Unavailable),
            "retained A cannot stand in for B"
        );
        direct_window
            .update(cx, |_, window, cx| {
                register(&replacement, window, cx);
            })
            .unwrap();
        let current = shells(cx).unwrap();
        assert_eq!(current.len(), 2);
        assert!(
            current
                .iter()
                .all(|weak| weak.entity_id() != old.entity_id())
        );
        assert!(
            current
                .iter()
                .any(|weak| weak.entity_id() == replacement.entity_id())
        );
    });
}
