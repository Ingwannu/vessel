use super::*;
use crate::{areas::session::Open, routes::Route, shell::language_tests};
use gpui::TestAppContext;
use vessel_app_view::layout::{HostAlias, LayoutKey, OpenIntent, Placement, Target};
use vessel_proto::SessionId;

#[gpui::test]
fn hidden_or_differently_selected_session_input_cannot_dispatch_or_clear_text(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |_shell, window, cx| {
        let (mut shell, mut commands) = language_tests::with_commands(window, cx);
        let session = SessionId::random();
        shell.areas.session.open = Some(Open {
            machine: "fixture".into(), session, title: String::new(), agent: String::new(),
            structured: true, live: true,
        });
        shell.areas.session.composer.set_text("retained input", window, cx);
        let key = LayoutKey::Session { machine: HostAlias::parse("fixture").unwrap(), session };
        shell.submit_prompt("retained input".into(), window, cx);
        assert!(commands.try_recv().is_err());
        assert_eq!(shell.areas.session.composer.text(cx), "retained input");
        shell.route = Route::Session;
        for target in [Target::Scripts, Target::Agent(SessionId::random())] {
            let _tab = shell.layout_open_target(&key, target, OpenIntent::New, Placement::Focused, window, cx).unwrap();
            assert_eq!(shell.composer_destination(), None);
            shell.submit_prompt("retained input".into(), window, cx);
            assert!(commands.try_recv().is_err());
            assert_eq!(shell.areas.session.composer.text(cx), "retained input");
            assert!(shell.areas.composer.sends.is_empty());
            assert!(shell.areas.session.resuming.is_none());
        }
        let _tab = shell.layout_open_target(&key, Target::Agent(session), OpenIntent::New, Placement::Focused, window, cx).unwrap();
        assert_eq!(shell.composer_destination(), Some(DraftKey::Session { session }));
        shell.submit_prompt("retained input".into(), window, cx);
        assert!(matches!(commands.try_recv().unwrap(), Command::Prompt { session: sent, text, .. } if sent == session && text == "retained input"));
        assert_eq!(shell.areas.session.composer.text(cx), "");
    });
}
