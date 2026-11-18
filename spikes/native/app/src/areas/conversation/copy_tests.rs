//! The Session actions menu's Copy agent id (TRACKER 804): Paseo's row and
//! its copied toast, "Copied Agent ID" (workspace-screen.tsx 2090; a Vessel
//! session is Paseo's agent), not a frame notice.
use crate::areas::conversation::MenuAction;
use crate::areas::conversation::area::Shown;
use crate::shell::Shell;
use gpui::TestAppContext;
use vessel_app_link::Link;
use vessel_app_view::toasts::ToastTone;
use vessel_proto::SessionId;

#[gpui::test]
fn copy_agent_id_copies_the_session_and_says_so_in_paseos_words(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, _sent) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.run_until_parked();
    let session = SessionId::random();
    shell.update_in(cx, |shell, window, cx| {
        shell.areas.conversation.shown = Some(Shown {
            machine: "isolated".to_owned(),
            session,
        });
        shell.menu_chose(MenuAction::CopyId, window, cx);
        let said = shell.areas.toasts.shown().cloned();
        let said = said.map(|toast| (toast.text, toast.tone));
        // Fluent isolates the label (U+2068 .. U+2069).
        let expected = "Copied \u{2068}Agent ID\u{2069}".to_owned();
        assert_eq!(said, Some((expected, ToastTone::Copied)));
        assert_eq!(shell.notice, None);
        let row = shell.t.lang.conversation().copy_agent_id();
        assert_eq!(row, "Copy agent id");
    });
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied, Some(session.to_string()));
}
