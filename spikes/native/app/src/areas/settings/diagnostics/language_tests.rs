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
            shell.notice = Some("another area's later notice".to_owned());
        },
        |shell| &shell.areas.session.composer,
        |shell| assert_eq!(shell.notice.as_deref(), Some("another area's later notice")),
    );
}

/// An enabled control from a previous frame must recheck the current run.
#[gpui::test]
fn an_old_copy_callback_cannot_publish_a_closed_pending_or_replaced_report(
    cx: &mut gpui::TestAppContext,
) {
    use vessel_app_kit::ui::ClipboardItem;
    use vessel_app_view::devices::MachineKey;
    use vessel_app_view::diagnostics::collection::{Problem, ReportText};
    enum Change {
        Closed,
        Refused,
        Pending,
        Replaced,
    }
    const BEFORE: &str = "clipboard before the stale callback";
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(BEFORE.to_owned()));
            let old = ReportText::new(REPORT).unwrap();
            for change in [
                Change::Closed,
                Change::Refused,
                Change::Pending,
                Change::Replaced,
            ] {
                let state = &mut shell.areas.settings.diagnostics;
                let ticket = state.tickets.next_ticket().unwrap();
                state.collection.start(ticket, old.clone(), []);
                match change {
                    Change::Closed => state.collection.close(),
                    Change::Refused => state.collection.refuse(Problem::Busy),
                    Change::Pending => {
                        let ticket = state.tickets.next_ticket().unwrap();
                        state.collection.start(
                            ticket,
                            old.clone(),
                            [(MachineKey::from_book("host"), Ok(()))],
                        );
                    }
                    Change::Replaced => {
                        let ticket = state.tickets.next_ticket().unwrap();
                        state.collection.start(
                            ticket,
                            ReportText::new("replacement report").unwrap(),
                            [],
                        );
                    }
                }
                shell.copy_diagnostic(&old, cx);
                assert!(shell.notice.is_none());
                assert!(
                    cx.read_from_clipboard()
                        .is_some_and(|item| item.text().is_some_and(|text| text == BEFORE))
                );
            }
        },
        |shell| &shell.areas.session.composer,
        |shell| assert!(shell.notice.is_none()),
    );
}
