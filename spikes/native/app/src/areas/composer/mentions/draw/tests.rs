use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;

#[gpui::test]
fn retained_draw_callback_cannot_choose_a_replacement_reply(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (target, _root) = super::super::tests::menu_fixture(shell, window, cx);
        let old = choice(shell, target);
        let clicked_before_query = callback(target, old.clone());
        let scope = old.request.as_ref().unwrap().scope.clone();
        shell
            .areas
            .session
            .composer
            .replace_all("@srcm", window, cx);
        let owner = shell.file_menu(target);
        let request = owner
            .borrow_mut()
            .model
            .observe(Some(scope), "@srcm", 5, std::time::Duration::ZERO)
            .unwrap()
            .unwrap();
        assert!(owner.borrow_mut().model.begin(request.ticket).is_some());
        // The renderer still shows A while this request loads. It captures
        // this request and A together; a reply must not turn that click into B.
        let clicked_while_loading = callback(target, choice(shell, target));
        assert!(owner.borrow_mut().model.arrived(
            request.ticket,
            Ok(vec![super::super::mentions::Suggestion {
                path: "src/other.rs".parse().unwrap(),
                kind: SuggestionKind::File,
            }]),
            std::time::Duration::ZERO
        ));
        clicked_before_query(shell, 0, window, cx);
        clicked_while_loading(shell, 0, window, cx);
        assert_eq!(shell.areas.session.composer.text(cx), "@srcm");
        callback(target, choice(shell, target))(shell, 0, window, cx);
        assert_eq!(shell.areas.session.composer.text(cx), "src/other.rs");
    });
}
