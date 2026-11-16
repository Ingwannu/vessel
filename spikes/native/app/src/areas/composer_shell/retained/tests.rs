//! Real canonical tickets and an actual field, with no fake identity allocator.
use super::*;
use crate::shell::{Shell, language_tests};
use gpui::TestAppContext;
use vessel_app_view::layout::{
    AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Target,
};
use vessel_app_view::residents::{PanelClaim, ResidentToken, Residents, SessionAddress};
use vessel_proto::{NodeId, SessionId, WorkspaceId};

fn admitted() -> (Residents, ResidentToken, FollowBinding) {
    let node = NodeId::random();
    let mut layout = Layout::default();
    let tab = layout
        .open(
            Target::NewTab,
            OpenIntent::New,
            Placement::Focused,
            AgentRole::Unknown,
            LayoutMode::SplitPanes,
        )
        .unwrap();
    let claim = PanelClaim {
        workspace: LayoutKey::Workspace {
            node,
            workspace: WorkspaceId::random(),
        },
        tab,
    };
    let mut registry = Residents::new();
    let token = registry
        .claim(
            SessionAddress {
                node,
                session: SessionId::random(),
            },
            claim,
            PaneId::MAIN,
        )
        .unwrap();
    let binding = registry.begin_follow(token).unwrap();
    (registry, token, binding)
}

#[test]
fn retirement_preserves_uncertain_effects_and_never_readmits_old_tickets() {
    let (mut registry, token, binding) = admitted();
    let ticket = registry
        .admit_operation(binding, OperationKind::Composer)
        .unwrap();
    let _foreign = registry
        .admit_operation(binding, OperationKind::Attachment)
        .unwrap();
    let mut owner = ResidentComposer::default();
    assert!(owner.track(binding, ticket));
    assert!(!owner.track(binding, ticket));
    assert_eq!(owner.effect_hold(), Some(HoldReason::PendingEffect));
    let notice = registry.retire_current(token).unwrap().unwrap();
    assert_eq!(notice.operations().len(), 2);
    owner.retired(&notice);
    assert_eq!(owner.effect_hold(), Some(HoldReason::UncertainEffect));
    assert!(!owner.settled(binding, ticket, Settled::Accepted));
    assert!(
        registry
            .settle_operation(binding, OperationKind::Composer, ticket)
            .is_err()
    );
    let fresh = registry.begin_follow(token).unwrap();
    let next = registry
        .admit_operation(fresh, OperationKind::Composer)
        .unwrap();
    assert!(owner.track(fresh, next));
    registry
        .settle_operation(fresh, OperationKind::Composer, next)
        .unwrap();
    assert!(owner.settled(fresh, next, Settled::Refused));
    assert_eq!(
        owner.effect_hold(),
        Some(HoldReason::UncertainEffect),
        "a new attempt cannot erase the previous uncertainty"
    );
}

#[test]
fn accepted_refused_and_unknown_settle_only_the_matching_admitted_effect() {
    for outcome in [Settled::Accepted, Settled::Refused, Settled::Unknown] {
        let (mut registry, _, binding) = admitted();
        let ticket = registry
            .admit_operation(binding, OperationKind::Composer)
            .unwrap();
        let mut owner = ResidentComposer::default();
        assert!(owner.track(binding, ticket));
        registry
            .settle_operation(binding, OperationKind::Composer, ticket)
            .unwrap();
        assert!(owner.settled(binding, ticket, outcome));
        assert!(!owner.settled(binding, ticket, outcome));
        assert_eq!(
            owner.effect_hold(),
            match outcome {
                Settled::Accepted | Settled::Refused => None,
                Settled::Unknown => Some(HoldReason::UncertainEffect),
            }
        );
    }
}

#[gpui::test]
fn snapshots_refuse_whitespace_focus_and_overlays_before_clean_transfer(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, window) = cx.add_window_view(Shell::without_io);
    window.run_until_parked();
    shell.update_in(window, |shell, window, cx| {
        window.blur(cx);
        let field = &shell.areas.draft.composer;
        let mut owner = ResidentComposer::default();
        let snapshot = owner.snapshot(field, window, cx).unwrap();
        assert_eq!(snapshot.surface(), owner.surface);
        field.set_text("  ", window, cx);
        assert!(matches!(
            owner.snapshot(field, window, cx),
            Err(HoldReason::DirtyInput)
        ));
        field.clear(window, cx);
        field.focus(window, cx);
        assert!(matches!(
            owner.snapshot(field, window, cx),
            Err(HoldReason::ActiveEditor)
        ));
        window.blur(cx);
        owner.surface.cover(Overlay::Dictation);
        assert!(matches!(
            owner.snapshot(field, window, cx),
            Err(HoldReason::Overlay)
        ));
        owner.surface.cover(Overlay::None);
        assert!(owner.snapshot(field, window, cx).is_ok());
    });
}
