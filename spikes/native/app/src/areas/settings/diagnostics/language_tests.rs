//! The real copy handler, no run started, and live language refresh without
//! I/O; a copy says so in Paseo's copied toast (TRACKER 804).
use vessel_app_view::toasts::ToastTone;

use crate::shell::language_tests;

const REPORT: &str = "Vessel app diagnostics\n  App version: fixture\n  Saved machines: 0";

#[gpui::test]
fn copied_toast_while_report_clipboard_and_input_stay_the_same(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, cx| {
            shell.copy_diagnostic(REPORT, cx);
            let said = shell
                .areas
                .toasts
                .shown()
                .map(|toast| (toast.text.as_str(), toast.tone));
            // Fluent isolates the label (U+2068 .. U+2069).
            let expected = "Copied \u{2068}diagnostic\u{2069}";
            assert_eq!(said, Some((expected, ToastTone::Copied)));
            assert_eq!(shell.notice, None);
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            assert!(shell.areas.settings.diagnostics.run.is_none());
        },
    );
    cx.update(|cx| {
        assert!(
            cx.read_from_clipboard()
                .is_some_and(|item| item.text().is_some_and(|text| text == REPORT))
        );
    });
}

/// Closing a diagnostics dialog gives focus back to what opened it (the
/// app reviewer's R1 on row 337: `DialogState::dismissed`), for each of the
/// three dialogs.
#[gpui::test]
fn each_dialog_gives_focus_back_to_its_opener_on_close(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(|window, cx| language_tests::with_commands(window, cx).0);
    shell.update_in(cx, |shell, window, cx| {
        let opener = cx.focus_handle();
        opener.focus(window, cx);
        shell.open_node_log("fixture", window, cx);
        assert!(!opener.is_focused(window), "the dialog takes focus");
        shell.close_node_log(window, cx);
        assert!(opener.is_focused(window), "Node logs gives it back");

        shell.run_diagnostic(window, cx);
        assert!(!opener.is_focused(window));
        shell.close_app_diagnostic(window, cx);
        assert!(
            opener.is_focused(window),
            "the app diagnostic gives it back"
        );
        assert!(shell.areas.settings.diagnostics.run.is_none());

        shell.open_agent_diagnostic(
            vessel_app_view::devices::MachineKey::from_book("fixture"),
            vessel_proto::HarnessRef::Known(vessel_proto::HarnessKind::ClaudeCode),
            window,
            cx,
        );
        assert!(!opener.is_focused(window));
        shell.close_agent_diagnostic(window, cx);
        assert!(
            opener.is_focused(window),
            "the agent's Diagnostic gives it back"
        );
    });
}
