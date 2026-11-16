//! Populated owner state crosses the sole registry, never selected globals.
use super::{ConversationArea, Origin};
use crate::areas::conversation::{ResidentSeat, TimelineSnapshot};
use crate::areas::{layout::residents::SessionResources, session::SessionView};
use crate::shell::{Shell, language_tests};
use gpui::{Context, TestAppContext, Window};
use vessel_app_kit::Reveal;
use vessel_app_view::layout::{
    AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Target,
};
use vessel_app_view::prefs::{ConversationDisplay, ReasoningExpansion};
use vessel_app_view::residents::{
    FollowBinding, HoldReason, OperationKind, OwnerReadiness, PanelClaim, ResidentProblem,
    SessionAddress, SessionContext, Visibility,
};
use vessel_app_view::timeline::EntryKind;
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, NodeId, SessionEvent, SessionId, WorkspaceId,
};

fn record(seq: u64, text: &str) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: AgentEvent::MessageChunk {
                content: ContentBlock::Text {
                    text: text.to_owned(),
                },
            },
        },
    }
}
fn claim(layout: &mut Layout, key: &LayoutKey) -> PanelClaim {
    PanelClaim {
        workspace: key.clone(),
        tab: layout
            .open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap(),
    }
}
fn text(area: &ConversationArea) -> &str {
    let EntryKind::Message { text } = &area.timeline.entries()[0].kind else {
        panic!("message")
    };
    text
}
struct Admitted {
    a: SessionContext,
    b: SessionContext,
    binding_a: FollowBinding,
    binding_b: FollowBinding,
    claim_a: PanelClaim,
}

fn admitted(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) -> Admitted {
    let residents = &mut shell.areas.session_residents;
    let node = NodeId::random();
    let key = LayoutKey::Workspace {
        node,
        workspace: WorkspaceId::random(),
    };
    let mut layout = Layout::default();
    let (claim_a, claim_b) = (claim(&mut layout, &key), claim(&mut layout, &key));
    let mut build = |_: &SessionContext| SessionResources {
        session: SessionView::new(window, cx),
        conversation: ConversationArea::with_presentation(
            ConversationDisplay::Full,
            ReasoningExpansion::Expanded,
            cx,
        ),
    };
    let a = residents
        .claim_session(
            SessionAddress {
                node,
                session: SessionId::random(),
            },
            &claim_a,
            PaneId::MAIN,
            &mut build,
        )
        .unwrap();
    let b = residents
        .claim_session(
            SessionAddress {
                node,
                session: SessionId::random(),
            },
            &claim_b,
            PaneId::MAIN,
            &mut build,
        )
        .unwrap();
    residents
        .present(a, &claim_a, PaneId::MAIN, Visibility::Visible)
        .unwrap();
    residents.focus(a, &claim_a).unwrap();
    let binding_a = residents.begin_follow(a, |_, _| {}).unwrap();
    let binding_b = residents.begin_follow(b, |_, _| {}).unwrap();
    let context_a = residents.context(a, &claim_a).unwrap();
    let context_b = residents.context(b, &claim_b).unwrap();
    Admitted {
        a: context_a,
        b: context_b,
        binding_a,
        binding_b,
        claim_a,
    }
}

fn assert_origin_is_separate(
    shell: &mut Shell,
    cx: &mut Context<'_, Shell>,
    seat: &ResidentSeat,
    context: SessionContext,
) -> ResidentSeat {
    let origin_seat = ResidentSeat::origin(context);
    assert!(
        origin_seat.get(shell).is_none(),
        "absent origin is not main"
    );
    let mut origin = ConversationArea::new(cx);
    assert!(origin.fold_records(&[record(1, "Origin")]));
    seat.get_mut(shell).unwrap().origin = Origin::Shown(Box::new(origin));
    assert_eq!(text(origin_seat.get(shell).unwrap()), "Origin");
    assert!(
        origin_seat
            .get_mut(shell)
            .unwrap()
            .fold_records(&[record(2, " retained")])
    );
    assert_eq!(text(origin_seat.get(shell).unwrap()), "Origin retained");
    assert_eq!(text(seat.get(shell).unwrap()), "A");
    origin_seat
}

fn assert_effect_settles_once(shell: &mut Shell, binding: FollowBinding) {
    let ticket = shell
        .areas
        .session_residents
        .admit_operation(binding, OperationKind::Timeline)
        .unwrap();
    assert!(
        shell
            .conversation_settled_scoped(binding, ticket, |area| {
                area.fold_records(&[record(2, " accepted")])
            })
            .unwrap()
    );
    assert!(matches!(
        shell.conversation_settled_scoped(binding, ticket, |area| area
            .fold_records(&[record(3, " duplicated")])),
        Err(ResidentProblem::UnadmittedOperation)
    ));
}

#[gpui::test]
fn scoped_hidden_records_and_effect_settlement_reach_only_the_admitted_timeline(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let Admitted {
            a,
            b,
            binding_a,
            binding_b,
            claim_a,
        } = admitted(shell, window, cx);
        assert!(
            shell
                .conversation_records_scoped(binding_a, &[record(1, "A")])
                .unwrap()
        );
        assert!(
            shell
                .conversation_records_scoped(binding_b, &[record(1, "B")])
                .unwrap()
        );
        let seat = ResidentSeat::main(a.clone());
        let captured = seat.clone();
        assert!(seat.get(shell).unwrap().turns.is_none());
        assert!(!seat.get(shell).unwrap().collapse);
        assert_eq!(
            seat.get(shell).unwrap().reasoning,
            ReasoningExpansion::Expanded
        );
        seat.get_mut(shell).unwrap().list.reveal(0, Reveal::Top);
        assert_eq!(text(captured.get(shell).unwrap()), "A");
        let origin_seat = assert_origin_is_separate(shell, cx, &seat, a.clone());
        assert!(matches!(
            shell.conversation_input_scoped(&b),
            Err(ResidentProblem::Held)
        ));
        assert_effect_settles_once(shell, binding_a);
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
        let hidden = shell
            .areas
            .session_residents
            .context(a.resident, &claim_a)
            .unwrap();
        assert!(matches!(
            shell.conversation_scoped(&a),
            Err(ResidentProblem::StaleEpoch)
        ));
        assert!(captured.get(shell).is_none());
        assert!(captured.root_get(shell).is_none());
        assert!(captured.get_mut(shell).is_none());
        assert!(origin_seat.get(shell).is_none());
        assert!(origin_seat.get_mut(shell).is_none());
        assert!(matches!(
            shell.conversation_input_scoped(&hidden),
            Err(ResidentProblem::Held)
        ));
        assert!(
            shell
                .conversation_records_scoped(binding_a, &[record(4, " hidden")])
                .unwrap()
        );
        let fresh = shell
            .areas
            .session_residents
            .begin_follow(a.resident, |_, _| {})
            .unwrap();
        assert!(matches!(
            shell.conversation_records_scoped(binding_a, &[record(9, " stale")]),
            Err(ResidentProblem::StaleBinding)
        ));
        assert!(
            shell
                .conversation_records_scoped(fresh, &[record(5, " fresh")])
                .unwrap()
        );
        assert_eq!(
            text(shell.conversation_scoped(&hidden).unwrap()),
            "A accepted hidden fresh"
        );
        assert_eq!(text(shell.conversation_scoped(&b).unwrap()), "B");
        assert!(
            shell.areas.conversation.timeline.entries().is_empty(),
            "selected global is not a fallback"
        );
    });
}
#[gpui::test]
fn a_clean_snapshot_moves_the_real_rows_disclosures_and_scroll_while_holds_return_it(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        let mut area = ConversationArea::new(cx);
        area.turns = None;
        area.collapse = false;
        let session = SessionId::random();
        let address = SessionAddress {
            node: NodeId::random(),
            session,
        };
        let mut residents = vessel_app_view::residents::Residents::new();
        let key = LayoutKey::Workspace {
            node: address.node,
            workspace: WorkspaceId::random(),
        };
        let panel = claim(&mut Layout::default(), &key);
        let token = residents
            .claim(address, panel.clone(), PaneId::MAIN)
            .unwrap();
        area.open("fake", session);
        let mut thought = record(1, "Reasoning");
        thought.event = SessionEvent::Agent {
            event: AgentEvent::ThoughtChunk {
                content: ContentBlock::Text {
                    text: "Reasoning".to_owned(),
                },
            },
        };
        assert!(area.take(&session, &[thought, record(2, "Reply")]));
        area.toggle_entry(0);
        area.list.reveal(0, Reveal::Top);
        let before = area.list.top();
        let Err(area) =
            area.into_timeline_snapshot(token, OwnerReadiness::Hold(HoldReason::ActiveEditor))
        else {
            panic!("navigation must hold")
        };
        assert_eq!(area.timeline.entries().len(), 2);
        let Ok(snapshot): Result<TimelineSnapshot, _> =
            area.into_timeline_snapshot(token, OwnerReadiness::Ready)
        else {
            panic!("idle owner can transfer")
        };
        assert!(!format!("{snapshot:?}").contains("Reasoning"));
        let foreign = vessel_app_view::residents::Residents::new()
            .claim(address, panel, PaneId::MAIN)
            .unwrap();
        let Err(snapshot) = snapshot.restore(foreign) else {
            panic!("another window must not receive entities")
        };
        let mut other = token;
        other.address.session = SessionId::random();
        let Err(snapshot) = snapshot.restore(other) else {
            panic!("another session must not receive rows")
        };
        other = token;
        other.address.node = NodeId::random();
        let Err(snapshot) = snapshot.restore(other) else {
            panic!("another node must not receive rows")
        };
        let restored = snapshot.restore(token).unwrap();
        assert_eq!(restored.timeline.entries().len(), 2);
        assert!(restored.entry_expanded(0));
        assert_eq!(restored.list.top(), before);
    });
}

fn thought_area(cx: &mut gpui::App, text: &str) -> ConversationArea {
    let mut area = ConversationArea::with_display(ConversationDisplay::Full, cx);
    let mut thought = record(1, text);
    thought.event = SessionEvent::Agent {
        event: AgentEvent::ThoughtChunk {
            content: ContentBlock::Text {
                text: text.to_owned(),
            },
        },
    };
    area.fold_records(&[thought]);
    area
}

fn origin_tree(cx: &mut gpui::App, first: &str, deep: &str) -> ConversationArea {
    let mut area = thought_area(cx, first);
    area.origin = Origin::Shown(Box::new(thought_area(cx, deep)));
    area.list.splice(0..0, area.head());
    area
}

fn assert_unrelated_origin_closed(shell: &Shell) {
    assert!(
        !crate::areas::conversation::Seat::SESSION
            .origin()
            .origin()
            .get(shell)
            .unwrap()
            .toggled
            .contains(&0)
    );
}

#[gpui::test]
fn cloned_row_seats_keep_recursive_origin_context_and_refuse_stale_dispatch(
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
            .conversation
            .open("fixture", a.resident.address.session);
        let first = origin_tree(cx, "A first origin", "A deep origin");
        shell
            .areas
            .session_residents
            .resources_mut(&a)
            .unwrap()
            .conversation
            .origin = Origin::Shown(Box::new(first));
        let unrelated = origin_tree(cx, "selected first origin", "selected deep origin");
        shell.areas.conversation.origin = Origin::Shown(Box::new(unrelated));
        let main = crate::areas::conversation::Seat::admitted_main(a.clone());
        assert!(main.get(shell).unwrap().shown.is_some());
        assert!(main.legacy_target(shell).is_none());
        let deep = main.origin().origin();
        let captured = deep.clone();
        assert!(std::ptr::eq(
            captured.root_get(shell).unwrap(),
            main.get(shell).unwrap()
        ));
        assert!(
            matches!(&deep.get(shell).unwrap().timeline.entries()[0].kind,
            EntryKind::Thought { text } if text == "A deep origin")
        );
        shell.toggle_entry(&captured, 0, cx);
        assert!(deep.get(shell).unwrap().toggled.contains(&0));
        assert_unrelated_origin_closed(shell);
        shell.toggle_entry(&captured, 0, cx);
        assert!(!deep.get(shell).unwrap().toggled.contains(&0));
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
        assert!(captured.get(shell).is_none());
        assert!(captured.get_mut(shell).is_none());
        shell.toggle_entry(&captured, 0, cx);
        assert_unrelated_origin_closed(shell);
    });
}

#[gpui::test]
fn retained_origin_protection_participates_in_the_parent_snapshot_vote(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut parent = ConversationArea::new(cx);
        let mut origin = ConversationArea::new(cx);
        origin.restarting = true;
        parent.origin = Origin::Shown(Box::new(origin));
        assert_eq!(
            parent.snapshot_readiness(OwnerReadiness::Ready),
            OwnerReadiness::Hold(HoldReason::PendingEffect)
        );
        let Origin::Shown(origin) = &mut parent.origin else {
            panic!("retained origin")
        };
        origin.restarting = false;
        assert_eq!(
            parent.snapshot_readiness(OwnerReadiness::Ready),
            OwnerReadiness::Ready
        );
    });
}

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
