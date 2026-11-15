//! Staged owner checks on the real canonical getter; no live scoped constructor.
use super::*;
use crate::areas::sessions::resident::LifecycleResources;
use crate::{
    areas::{
        conversation::ConversationArea,
        layout::residents::{SessionAdmission, SessionOpening, SessionResidents, SessionResources},
        session::{Open, SessionView},
    },
    shell::{Shell, language_tests},
};
use gpui::TestAppContext;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::{
    Card,
    connection::{ConnectionEpochs, MachineKey},
    layout::{LookupProblem, WorkspaceLocation, WorkspaceLookup},
    residents::{HoldReason, OperationKind, OwnerReadiness, Visibility},
    sessions::LifecycleAction,
};
use vessel_proto::{HarnessKind, NodeId, SessionId, SessionMode, SessionSpec, WorkspaceId};

struct Registry(WorkspaceId);
impl WorkspaceLookup for Registry {
    fn session_workspace(
        &self,
        _: NodeId,
        _: SessionId,
    ) -> Result<Option<WorkspaceId>, LookupProblem> {
        Ok(Some(self.0))
    }
    fn workspace_location(
        &self,
        _: NodeId,
        _: WorkspaceId,
    ) -> Result<WorkspaceLocation, LookupProblem> {
        Err(LookupProblem::Unavailable)
    }
}
fn received_snapshot(
    residents: &mut SessionResidents,
    admission: &SessionAdmission,
) -> Result<Snapshot, Problem> {
    residents
        .receive(admission.binding, |resources| {
            Snapshot::received(resources, admission)
        })
        .map_err(Problem::Scope)?
}
fn admitted(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> SessionAdmission {
    let info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![OptionChoice {
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
    let mut epochs = ConnectionEpochs::default();
    let connection = epochs
        .admit(MachineKey::from_book("fixture"), address.node)
        .unwrap();
    let registry = Registry(WorkspaceId::random());
    let opening = SessionOpening::resolve(&registry, connection, info.id).unwrap();
    shell
        .admit_session_open(
            &opening,
            &registry,
            |_, window, cx| {
                // Existing constructor is an isolated test factory, never scoped input.
                let mut session = SessionView::new(window, cx);
                session.open = Some(Open::of(&Card::from_session("fixture", &info)));
                let mut conversation = ConversationArea::new(cx);
                conversation.open("fixture", info.id);
                conversation.info = Some(Box::new(info));
                SessionResources {
                    session,
                    conversation,
                }
            },
            |_, _| panic!("first admission retired"),
            window,
            cx,
        )
        .unwrap()
}

#[gpui::test]
fn lifecycle_stays_loading_until_exact_scoped_hydration_and_transfers_only_clean_snapshot(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let admission = admitted(shell, window, cx);
        let context = admission.context.clone();
        let residents = &mut shell.areas.session_residents;
        let mut owner = LifecycleResources::staged();
        assert_eq!(owner.readiness(), OwnerReadiness::Hold(HoldReason::Loading));
        let mut conflicting = context.clone();
        conflicting.resident.address.node = NodeId::random();
        assert!(residents.resources(&conflicting).is_err());
        let wrong = SessionAdmission {
            connection: admission.connection.clone(),
            context: conflicting,
            binding: admission.binding,
        };
        assert!(matches!(
            received_snapshot(residents, &wrong),
            Err(Problem::WrongResource)
        ));
        let snapshot = received_snapshot(residents, &admission).unwrap();
        assert_eq!(snapshot.info().id, context.resident.address.session);
        assert_eq!(snapshot.choices().len(), 1);
        assert_eq!(snapshot.choices(), snapshot.info().spec.mode.options());
        owner.hydrate(snapshot).unwrap();
        assert_eq!(owner.readiness(), OwnerReadiness::Ready);
        let clean = owner.take_clean_snapshot().unwrap();
        assert_eq!(clean.resident(), context.resident);
        assert_eq!(owner.readiness(), OwnerReadiness::Hold(HoldReason::Loading));
        // The minter already focused this claim. Change presentation for real;
        // focusing the same claim is intentionally idempotent.
        residents
            .present(
                context.resident,
                &context.claim,
                context.pane,
                Visibility::RetainedHidden,
            )
            .unwrap();
        assert!(residents.resources(&context).is_err());
        let received = received_snapshot(residents, &admission).unwrap();
        assert_eq!(received.resident(), context.resident);
        assert_eq!(
            residents
                .context(context.resident, &context.claim)
                .unwrap()
                .visibility,
            Visibility::RetainedHidden
        );
        // Only a local presentation transition restores visible input. Network
        // hydration above neither refreshed its context nor acquired focus.
        residents
            .present(
                context.resident,
                &context.claim,
                context.pane,
                Visibility::Visible,
            )
            .unwrap();
        residents.focus(context.resident, &context.claim).unwrap();
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
            received_snapshot(residents, &admission),
            Err(Problem::WrongResource)
        ));
        residents.resources_mut(&context).unwrap().conversation.info = None;
        assert!(matches!(
            received_snapshot(residents, &admission),
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
        let admission = admitted(shell, window, cx);
        let context = admission.context.clone();
        let residents = &mut shell.areas.session_residents;
        let mut owner = LifecycleResources::staged();
        owner
            .hydrate(received_snapshot(residents, &admission).unwrap())
            .unwrap();
        let binding = admission.binding;
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
            received_snapshot(residents, &admission),
            Err(Problem::Scope(_))
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
        let admission = admitted(shell, window, cx);
        let context = admission.context.clone();
        let residents = &mut shell.areas.session_residents;
        let mut owner = LifecycleResources::staged();
        owner
            .hydrate(received_snapshot(residents, &admission).unwrap())
            .unwrap();
        let binding = admission.binding;
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

#[gpui::test]
fn exact_no_send_releases_only_pending_ui_while_uncertainty_preserves_input(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    for disposition in [Disposition::NoSend, Disposition::Uncertain] {
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, window, cx| {
            let admission = admitted(shell, window, cx);
            let context = admission.context.clone();
            let residents = &mut shell.areas.session_residents;
            let mut owner = LifecycleResources::staged();
            owner
                .hydrate(received_snapshot(residents, &admission).unwrap())
                .unwrap();
            let binding = admission.binding;
            let decision = owner.prepare(binding, LifecycleAction::Fresh).unwrap();
            let ticket = residents
                .admit_operation(binding, OperationKind::Lifecycle)
                .unwrap();
            owner.sent(decision, ticket).unwrap();
            residents
                .resources_mut(&context)
                .unwrap()
                .session
                .composer
                .set_text(" keep draft ", window, cx);
            let stale = residents
                .admit_operation(binding, OperationKind::Lifecycle)
                .unwrap();
            assert!(!owner.failed(binding, stale, disposition));
            assert_eq!(
                owner.readiness(),
                OwnerReadiness::Hold(HoldReason::PendingEffect)
            );
            assert!(
                residents
                    .settle(binding, OperationKind::Lifecycle, ticket, |_| owner.failed(
                        binding,
                        ticket,
                        disposition
                    ))
                    .unwrap()
            );
            let expected = if matches!(disposition, Disposition::NoSend) {
                OwnerReadiness::Ready
            } else {
                OwnerReadiness::Hold(HoldReason::UncertainEffect)
            };
            assert_eq!(owner.readiness(), expected);
            assert_eq!(owner.acknowledged(binding, ticket), None);
            assert!(
                residents
                    .settle(binding, OperationKind::Lifecycle, ticket, |_| panic!(
                        "duplicate settled"
                    ))
                    .is_err()
            );
            assert_eq!(
                residents
                    .resources(&context)
                    .unwrap()
                    .session
                    .composer
                    .text(cx),
                " keep draft "
            );
        });
    }
}
