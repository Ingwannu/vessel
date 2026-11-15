//! Real book and fields exercise refused admission plus independent retirement.
use super::*;
use crate::areas::layout::residents::{SessionAdmission, SessionOpening, SessionOpeningProblem};
use crate::areas::{conversation::ConversationArea, session::SessionView};
use crate::shell::language_tests;
use gpui::TestAppContext;
use std::cell::Cell;
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::layout::{WorkspaceLocation, WorkspaceLookup};
use vessel_app_view::residents::{OperationKind, PanelFocus};
use vessel_proto::{NodeId, WorkspaceId};

pub(super) struct Registry(pub(super) WorkspaceId);
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
pub(super) fn connection(epochs: &mut ConnectionEpochs, node: NodeId) -> ConnectionContext {
    epochs
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap()
}
pub(super) fn build(
    context: &SessionContext,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> SessionResources {
    assert_eq!(
        context.resident.address.node,
        match context.claim.workspace {
            LayoutKey::Workspace { node, .. } => node,
            LayoutKey::Session { .. } => panic!("rejected work must not execute"),
        }
    );
    SessionResources {
        session: SessionView::new(window, cx),
        conversation: ConversationArea::new(cx),
    }
}

#[gpui::test]
fn actual_tab_claim_exists_before_factory_and_same_session_keeps_its_field(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let mut epochs = ConnectionEpochs::default();
        let original = connection(&mut epochs, NodeId::random());
        let registry = Registry(WorkspaceId::random());
        let opening =
            SessionOpening::resolve(&registry, original.clone(), SessionId::random()).unwrap();
        let first: SessionAdmission = shell
            .admit_session_open(&opening, &registry, build, |_, _| {}, window, cx)
            .unwrap();
        assert_eq!(first.connection, original);
        assert_eq!(first.context.focus, PanelFocus::Focused);
        assert!(shell.layout_tab_exists(&first.context.claim.workspace, first.context.claim.tab));
        let field = shell
            .areas
            .session_residents
            .resources(&first.context)
            .unwrap()
            .session
            .composer
            .clone();
        field.set_text("retained input", window, cx);
        let called = Cell::new(false);
        let again = shell
            .admit_session_open(
                &opening,
                &registry,
                |_, _, _| {
                    called.set(true);
                    panic!("existing resources must not be reconstructed")
                },
                |_, retired| assert_eq!(retired.binding(), first.binding),
                window,
                cx,
            )
            .unwrap();
        assert!(!called.get());
        assert_eq!(again.context.resident, first.context.resident);
        assert_ne!(again.binding, first.binding);
        let current = shell
            .areas
            .session_residents
            .resources(&again.context)
            .unwrap();
        assert_eq!(current.session.composer.id(), field.id());
        assert_eq!(current.session.composer.text(cx), "retained input");
        assert!(
            shell
                .areas
                .session_residents
                .receive(first.binding, |_| ())
                .is_err()
        );
    });
}

struct NotHydrated;
impl WorkspaceLookup for NotHydrated {
    fn session_workspace(
        &self,
        _: NodeId,
        _: SessionId,
    ) -> Result<Option<WorkspaceId>, LookupProblem> {
        Err(LookupProblem::NotHydrated)
    }
    fn workspace_location(
        &self,
        _: NodeId,
        _: WorkspaceId,
    ) -> Result<WorkspaceLocation, LookupProblem> {
        Err(LookupProblem::NotHydrated)
    }
}
#[test]
fn missing_registry_never_manufactures_a_legacy_session_key() {
    let mut epochs = ConnectionEpochs::default();
    let context = connection(&mut epochs, NodeId::random());
    assert!(matches!(
        SessionOpening::resolve(&NotHydrated, context, SessionId::random()),
        Err(SessionOpeningProblem::Lookup(LookupProblem::NotHydrated))
    ));
}

#[gpui::test]
fn refused_switch_does_not_delay_retirement_or_clear_protected_input(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let mut epochs = ConnectionEpochs::default();
        let original = connection(&mut epochs, NodeId::random());
        let registry = Registry(WorkspaceId::random());
        let opening =
            SessionOpening::resolve(&registry, original.clone(), SessionId::random()).unwrap();
        let first = shell
            .admit_session_open(&opening, &registry, build, |_, _| {}, window, cx)
            .unwrap();
        shell
            .areas
            .session_residents
            .resources(&first.context)
            .unwrap()
            .session
            .composer
            .set_text("  protected  ", window, cx);
        let ticket = shell
            .areas
            .session_residents
            .admit_operation(first.binding, OperationKind::Composer)
            .unwrap();
        let other =
            SessionOpening::resolve(&registry, original.clone(), SessionId::random()).unwrap();
        let before = shell.areas.layout.book.borrow().clone();
        let result = shell.admit_session_open(
            &other,
            &registry,
            |_, _, _| panic!("rejected work must not execute"),
            |_, _| panic!("rejected work must not execute"),
            window,
            cx,
        );
        assert!(matches!(
            result,
            Err(SessionOpeningProblem::Resident(ResidentProblem::Capacity))
        ));
        assert_eq!(*shell.areas.layout.book.borrow(), before);
        shell
            .areas
            .session_residents
            .receive(first.binding, |_| ())
            .unwrap();
        shell
            .areas
            .session_residents
            .connection_retired(&original, |resources, notice| {
                assert_eq!(notice.binding(), first.binding);
                assert_eq!(notice.operations().first().unwrap().ticket(), ticket);
                resources.session.resume_error = Some("uncertain original send".into());
            })
            .unwrap();
        let residents = &mut shell.areas.session_residents;
        assert!(
            residents
                .receive(first.binding, |_| panic!("rejected work must not execute"))
                .is_err()
        );
        assert!(
            residents
                .settle(first.binding, OperationKind::Composer, ticket, |_| panic!(
                    "rejected work must not execute"
                ))
                .is_err()
        );
        let kept = residents.resources(&first.context).unwrap();
        assert_eq!(kept.session.composer.text(cx), "  protected  ");
        assert_eq!(
            kept.session.resume_error.as_deref(),
            Some("uncertain original send")
        );
    });
}

#[gpui::test]
fn old_connection_notice_cannot_retire_same_node_successor(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let mut epochs = ConnectionEpochs::default();
        let node = NodeId::random();
        let original = connection(&mut epochs, node);
        let registry = Registry(WorkspaceId::random());
        let session = SessionId::random();
        let first = SessionOpening::resolve(&registry, original.clone(), session).unwrap();
        let _first = shell
            .admit_session_open(&first, &registry, build, |_, _| {}, window, cx)
            .unwrap();
        let fresh = connection(&mut epochs, node);
        let next = SessionOpening::resolve(&registry, fresh.clone(), session).unwrap();
        let admitted = shell
            .admit_session_open(
                &next,
                &registry,
                |_, _, _| panic!("rejected work must not execute"),
                |_, _| {},
                window,
                cx,
            )
            .unwrap();
        assert_eq!(admitted.connection, fresh);
        shell
            .areas
            .session_residents
            .connection_retired(&original, |_, _| panic!("rejected work must not execute"))
            .unwrap();
        shell
            .areas
            .session_residents
            .receive(admitted.binding, |_| ())
            .unwrap();
    });
}

#[gpui::test]
fn deferred_open_refuses_changed_membership_before_allocating_a_tab(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let mut epochs = ConnectionEpochs::default();
        let original = connection(&mut epochs, NodeId::random());
        let opening = SessionOpening::resolve(
            &Registry(WorkspaceId::random()),
            original,
            SessionId::random(),
        )
        .unwrap();
        let changed = Registry(WorkspaceId::random());
        let result = shell.admit_session_open(
            &opening,
            &changed,
            |_, _, _| panic!("rejected work must not execute"),
            |_, _| panic!("rejected work must not execute"),
            window,
            cx,
        );
        assert!(matches!(
            result,
            Err(SessionOpeningProblem::Resident(
                ResidentProblem::OwnerConflict
            ))
        ));
        assert!(shell.areas.layout.book.borrow().is_none());
        assert!(shell.areas.session_residents.entries.is_empty());
    });
}
