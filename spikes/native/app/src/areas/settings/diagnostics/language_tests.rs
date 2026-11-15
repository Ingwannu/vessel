//! The real copy handler, retained report and live language refresh without I/O.
use crate::shell::language_tests;

const REPORT: &str = "Vessel app diagnostics\n  App version: fixture\n  Saved machines: 0";

#[gpui::test]
fn copied_notice_rewords_while_report_clipboard_and_input_stay_the_same(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, cx| {
            let state = &mut shell.areas.settings.diagnostics;
            let ticket = state.tickets.next_ticket().unwrap();
            state.collection.start(
                ticket,
                vessel_app_view::diagnostics::collection::ReportText::new(REPORT).unwrap(),
                [],
            );
            shell.copy_diagnostic(
                &vessel_app_view::diagnostics::collection::ReportText::new(REPORT).unwrap(),
                cx,
            );
            assert_eq!(shell.notice.as_deref(), Some("Diagnostic copied."));
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            assert_eq!(shell.notice.as_deref(), Some("진단을 복사했습니다."));
            assert_eq!(
                shell
                    .areas
                    .settings
                    .diagnostics
                    .collection
                    .report()
                    .map(vessel_app_view::diagnostics::collection::ReportText::as_str),
                Some(REPORT)
            );
        },
    );
    cx.update(|cx| {
        assert!(
            cx.read_from_clipboard()
                .is_some_and(|item| item.text().is_some_and(|text| text == REPORT))
        );
    });
}

#[gpui::test]
fn an_older_copy_notice_does_not_replace_another_areas_later_notice(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, cx| {
            shell.copy_diagnostic(
                &vessel_app_view::diagnostics::collection::ReportText::new(REPORT).unwrap(),
                cx,
            );
            shell.notice = Some("another area's later notice".to_owned());
        },
        |shell| &shell.areas.session.composer,
        |shell| assert_eq!(shell.notice.as_deref(), Some("another area's later notice")),
    );
}
