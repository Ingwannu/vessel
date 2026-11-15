//! Staged owner checks on the real canonical getter; no live scoped constructor.
use super::*;
use crate::{
    areas::{
        conversation::ConversationArea,
        layout::residents::SessionResources,
        session::{Open, SessionView},
    },
    shell::{Shell, language_tests},
};
use gpui::TestAppContext;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::{
    Card,
    layout::{AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Target},
    residents::{HoldReason, OperationKind, OwnerReadiness, PanelClaim},
    sessions::LifecycleAction,
};
use vessel_proto::{HarnessKind, NodeId, SessionId, SessionMode, SessionSpec, WorkspaceId};

fn admitted(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) -> SessionContext {
    let info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![vessel_proto::OptionChoice {
                    id: vessel_proto::OptionId::new("fast"),
                    value: vessel_proto::ChoiceValue::Toggle { on: true },
                }],
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    let address = vessel_app_view::residents::SessionAddress {
        node: NodeId::random(),
        session: info.id,
    };
    let key = LayoutKey::Workspace {
        node: address.node,
        workspace: WorkspaceId::random(),
    };
    let mut layout = Layout::default();
    let tab = layout
        .open(
            Target::Agent(info.id),
            OpenIntent::New,
            Placement::Focused,
            AgentRole::Unknown,
            LayoutMode::SplitPanes,
        )
        .unwrap();
    let claim = PanelClaim {
        workspace: key,
        tab,
    };
    let token = shell
        .areas
        .session_residents
        .claim_session(address, &claim, PaneId::MAIN, |_| {
            // Existing constructor is an isolated test factory, never scoped input.
            let mut session = SessionView::new(window, cx);
            let mut open = Open::of(&Card::from_session("fixture", &info));
            open.binding = Some(vessel_app_link::sessions::Binding {
                host: vessel_app_view::machine_book::Name::parse("fixture").unwrap(),
                address,
            });
            session.open = Some(open);
            let mut conversation = ConversationArea::new(cx);
            conversation.open("fixture", info.id);
            conversation.info = Some(Box::new(info));
            SessionResources {
                session,
                conversation,
            }
        })
        .unwrap();
    shell
        .areas
        .session_residents
        .context(token, &claim)
        .unwrap()
}

#[gpui::test]
fn lifecycle_stays_loading_until_exact_scoped_hydration_and_transfers_only_clean_snapshot(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let context = admitted(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let mut owner = LifecycleResources::staged();
        assert_eq!(owner.readiness(), OwnerReadiness::Hold(HoldReason::Loading));
        let mut conflicting = context.clone();
        conflicting.resident.address.node = NodeId::random();
        assert!(matches!(
            Snapshot::capture(residents, &conflicting),
            Err(Problem::Scope(_))
        ));
        let snapshot = Snapshot::capture(residents, &context).unwrap();
        assert_eq!(snapshot.info().id, context.resident.address.session);
        assert_eq!(snapshot.choices().len(), 1);
        assert_eq!(snapshot.choices(), snapshot.info().spec.mode.options());
        owner.hydrate(snapshot).unwrap();
        assert_eq!(owner.readiness(), OwnerReadiness::Ready);
        let clean = owner.take_clean_snapshot().unwrap();
        assert_eq!(clean.resident(), context.resident);
        assert_eq!(owner.readiness(), OwnerReadiness::Hold(HoldReason::Loading));
        residents.focus(context.resident, &context.claim).unwrap();
        assert!(matches!(
            Snapshot::capture(residents, &context),
            Err(Problem::Scope(_))
        ));
        let binding = residents
            .begin_follow(context.resident, |_, _| panic!("first attach retired"))
            .unwrap();
        let received = residents
            .receive(binding, |resources| Snapshot::received(resources, binding))
            .unwrap()
            .unwrap();
        assert_eq!(received.resident(), context.resident);
        // A local caller refreshes its same claim; network hydration above did not.
        let context = residents.context(context.resident, &context.claim).unwrap();
        residents
            .resources_mut(&context)
            .unwrap()
            .conversation
            .info
            .as_mut()
            .unwrap()
            .id = SessionId::random();
        assert!(matches!(
            Snapshot::capture(residents, &context),
            Err(Problem::WrongResource)
        ));
        residents.resources_mut(&context).unwrap().conversation.info = None;
        assert!(matches!(
            Snapshot::capture(residents, &context),
            Err(Problem::Loading)
        ));
    });
}

#[gpui::test]
fn lifecycle_retirement_keeps_input_and_never_restores_acknowledgement_authority(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let context = admitted(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let mut owner = LifecycleResources::staged();
        owner
            .hydrate(Snapshot::capture(residents, &context).unwrap())
            .unwrap();
        let binding = residents
            .begin_follow(context.resident, |_, _| panic!("first attach retired"))
            .unwrap();
        let decision = owner.prepare(binding, LifecycleAction::Fresh).unwrap();
        let ticket = residents
            .admit_operation(binding, OperationKind::Lifecycle)
            .unwrap();
        owner.sent(decision, ticket).unwrap();
        assert_eq!(
            owner.readiness(),
            OwnerReadiness::Hold(HoldReason::PendingEffect)
        );
        residents
            .resources_mut(&context)
            .unwrap()
            .session
            .composer
            .set_text(" retained input ", window, cx);
        residents
            .retire_follow(binding, |_, notice| owner.retired(notice))
            .unwrap();
        assert_eq!(
            owner.readiness(),
            OwnerReadiness::Hold(HoldReason::UncertainEffect)
        );
        let uncertain = owner.uncertain().unwrap();
        assert_eq!(uncertain.binding, binding);
        assert_eq!(uncertain.ticket, ticket);
        assert_eq!(uncertain.action, LifecycleAction::Fresh);
        assert!(matches!(owner.take_clean_snapshot(), Err(Problem::Held)));
        assert!(matches!(
            owner.hydrate(Snapshot::capture(residents, &context).unwrap()),
            Err(Problem::Held)
        ));
        assert!(
            residents
                .settle(binding, OperationKind::Lifecycle, ticket, |_| owner
                    .acknowledged(binding, ticket))
                .is_err()
        );
        assert_eq!(owner.acknowledged(binding, ticket), None);
        assert_eq!(
            residents
                .resources(&context)
                .unwrap()
                .session
                .composer
                .text(cx),
            " retained input "
        );
    });
}

#[gpui::test]
fn exact_decision_and_single_use_canonical_settlement_gate_completion(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let context = admitted(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let mut owner = LifecycleResources::staged();
        owner
            .hydrate(Snapshot::capture(residents, &context).unwrap())
            .unwrap();
        let binding = residents
            .begin_follow(context.resident, |_, _| panic!("first attach retired"))
            .unwrap();
        let old = owner.prepare(binding, LifecycleAction::Archive).unwrap();
        residents
            .retire_follow(binding, |_, notice| owner.retired(notice))
            .unwrap();
        assert_eq!(owner.readiness(), OwnerReadiness::Hold(HoldReason::Overlay));
        owner.dismiss(old);
        let fresh = residents
            .begin_follow(context.resident, |_, _| panic!("already retired"))
            .unwrap();
        let decision = owner.prepare(fresh, LifecycleAction::Exit).unwrap();
        assert_ne!(old, decision);
        owner.dismiss(old);
        let ticket = residents
            .admit_operation(fresh, OperationKind::Lifecycle)
            .unwrap();
        assert_eq!(owner.sent(old, ticket), Err(Problem::Held));
        owner.sent(decision, ticket).unwrap();
        assert_eq!(
            residents
                .settle(fresh, OperationKind::Lifecycle, ticket, |_| owner
                    .acknowledged(fresh, ticket))
                .unwrap(),
            Some(LifecycleAction::Exit)
        );
        assert!(
            residents
                .settle(fresh, OperationKind::Lifecycle, ticket, |_| owner
                    .acknowledged(fresh, ticket))
                .is_err()
        );
        assert_eq!(owner.readiness(), OwnerReadiness::Ready);
    });
}
