//! The global admission query is read-only and agrees with the actual key handler.
use super::*;
use crate::{areas::session::Open, routes::Route, shell::language_tests};
use gpui::TestAppContext;
use vessel_app_kit::field::composer_hooks::KeyAnswer;
use vessel_proto::SessionId;

#[gpui::test]
fn menu_keys_are_scoped_and_querying_never_changes_input_or_selection(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        let session = SessionId::random();
        let target = DraftKey::Session { session };
        shell.route = Route::Session;
        shell.areas.session.open = Some(Open {
            machine: "fixture".to_owned(),
            session,
            title: String::new(),
            agent: String::new(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        shell
            .layout_open_target(
                &vessel_app_view::layout::LayoutKey::Session {
                    machine: "fixture".parse().unwrap(),
                    session,
                },
                vessel_app_view::layout::Target::Agent(session),
                vessel_app_view::layout::OpenIntent::Reveal,
                vessel_app_view::layout::Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let field = shell.areas.session.composer.clone();
        field.replace_all("/q", window, cx);
        let before = (field.text(cx), field.selection(cx));
        for key in [
            ComposerKey::Escape,
            ComposerKey::Tab,
            ComposerKey::Enter,
            ComposerKey::Up,
            ComposerKey::Down,
        ] {
            assert!(shell.composer_command_menu_owns_key(target, key, cx));
            assert!(!shell.composer_command_menu_owns_key(DraftKey::New, key, cx));
            assert!(!shell.composer_command_menu_owns_key(
                DraftKey::Session {
                    session: SessionId::random()
                },
                key,
                cx
            ));
        }
        assert_eq!((field.text(cx), field.selection(cx)), before);
        assert_eq!(
            shell.slash_key(Over::Session, ComposerKey::Escape, window, cx),
            KeyAnswer::Taken
        );
        assert_eq!(field.text(cx), "");
        assert!(!shell.composer_command_menu_owns_key(target, ComposerKey::Escape, cx));
        field.replace_all("/no-such-command", window, cx);
        assert!(!shell.composer_command_menu_owns_key(target, ComposerKey::Tab, cx));
        assert_eq!(
            shell.slash_key(Over::Session, ComposerKey::Tab, window, cx),
            KeyAnswer::Pass
        );
        field.replace_all("/q", window, cx);
        field.set_editability(vessel_app_kit::Editability::ReadOnly, cx);
        assert!(!shell.composer_command_menu_owns_key(target, ComposerKey::Escape, cx));
    });
}
