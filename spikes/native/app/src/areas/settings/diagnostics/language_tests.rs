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
