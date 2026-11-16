//! Full renderer callbacks and concrete owner settlement, beyond the baseline carrier.
use super::{Admitted, admitted, record, text, thought_area};
use crate::shell::{Shell, language_tests};
use gpui::TestAppContext;
use vessel_app_view::layout::PaneId;
use vessel_app_view::residents::{OperationKind, Visibility};
use vessel_proto::EventSeq;

#[gpui::test]
fn full_a7_compact_sheet_dispatch_keeps_the_original_resident_and_refuses_hidden_context(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let Admitted { a, claim_a, .. } = admitted(shell, window, cx);
        shell
            .areas
            .session_residents
            .resources_mut(&a)
            .unwrap()
            .conversation = thought_area(cx, "A reasoning");
        shell.areas.conversation = thought_area(cx, "unrelated reasoning");
        let captured = crate::areas::conversation::Seat::admitted_main(a.clone());
        assert!(shell.entry_sheet(&captured, 0, window, cx));
        assert!(captured.get(shell).unwrap().tool_sheet.snapshot_busy());
        assert!(!shell.areas.conversation.tool_sheet.snapshot_busy());
        shell
            .areas
            .session_residents
            .present(
                a.resident,
                &claim_a,
                PaneId::MAIN,
                Visibility::RetainedHidden,
            )
            .unwrap();
        assert!(!shell.entry_sheet(&captured, 0, window, cx));
        assert!(!shell.run_sheet(&captured, 0, window, cx));
        assert!(!shell.areas.conversation.tool_sheet.snapshot_busy());
        let hidden = shell
            .areas
            .session_residents
            .context(a.resident, &claim_a)
            .unwrap();
        assert!(
            shell
                .areas
                .session_residents
                .resources(&hidden)
                .unwrap()
                .conversation
                .tool_sheet
                .snapshot_busy()
        );
    });
}

#[gpui::test]
fn owner_correlation_is_checked_before_ticket_consume_and_hidden_mutation_stays_local(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let Admitted {
            a,
            b,
            binding_a,
            claim_a,
            ..
        } = admitted(shell, window, cx);
        shell
            .conversation_records_scoped(binding_a, &[record(1, "A")])
            .unwrap();
        let key = shell.conversation_scoped(&a).unwrap().timeline.entries()[0]
            .key
            .clone();
        let ticket = shell
            .areas
            .session_residents
            .admit_operation(binding_a, OperationKind::Timeline)
            .unwrap();
        shell
            .areas
            .session_residents
            .present(
                a.resident,
                &claim_a,
                PaneId::MAIN,
                Visibility::RetainedHidden,
            )
            .unwrap();
        let refused = shell.conversation_settle_if_scoped::<()>(
            binding_a,
            ticket,
            |area| {
                area.timeline.entries().first().is_some_and(|entry| {
                    entry.key == vessel_app_view::timeline::EntryKey::Record(EventSeq::new(99))
                })
            },
            |_| panic!("wrong owner correlation must never mutate"),
        );
        assert!(refused.unwrap().is_none());
        let accepted = shell.conversation_settle_if_scoped(
            binding_a,
            ticket,
            |area| {
                area.timeline
                    .entries()
                    .first()
                    .is_some_and(|entry| entry.key == key)
            },
            |area| area.fold_records(&[record(2, " accepted")]),
        );
        assert_eq!(accepted.unwrap(), Some(true));
        let hidden = shell
            .areas
            .session_residents
            .context(a.resident, &claim_a)
            .unwrap();
        assert_eq!(
            text(shell.conversation_scoped(&hidden).unwrap()),
            "A accepted"
        );
        assert!(
            shell
                .conversation_scoped(&b)
                .unwrap()
                .timeline
                .entries()
                .is_empty()
        );
        assert!(
            shell
                .conversation_settle_if_scoped::<()>(
                    binding_a,
                    ticket,
                    |_| true,
                    |_| panic!("spent ticket must never mutate")
                )
                .is_err()
        );
    });
}
