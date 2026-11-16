//! Actual staged minter invokes the scoped field factory after the real claim.
//! This draws the retained field directly, not a converted agent pane or wire.
use super::*;
use gpui::{AppContext as _, Entity, IntoElement, ParentElement as _, Render};
use std::{cell::RefCell, rc::Rc};
use vessel_app_kit::{Scheme, TextField, Tokens};
use vessel_app_view::prefs::ConversationDisplay;
use vessel_app_view::residents::{
    HoldReason, OwnerReadiness, StopAction, StopCompletion, StopSettlement,
};
use vessel_proto::{HarnessKind, SessionInfo, SessionMode, SessionSpec};

fn hydrate_lifecycle(residents: &mut SessionResidents, admission: &SessionAdmission) {
    let info = SessionInfo::new(
        admission.binding.resident.address.session,
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    residents
        .receive(admission.binding, |resources| {
            resources.session.open = Some(crate::areas::session::Open::of(
                &vessel_app_view::Card::from_session("fixture", &info),
            ));
            resources.conversation.open("fixture", info.id);
            resources.conversation.info = Some(Box::new(info));
            let snapshot =
                crate::areas::session_lifecycle::Snapshot::received(resources, admission).unwrap();
            resources.lifecycle.hydrate(snapshot).unwrap();
            assert_eq!(resources.lifecycle.readiness(), OwnerReadiness::Ready);
        })
        .unwrap();
}

type Observed = Rc<RefCell<Vec<SessionContext>>>;
struct Probe {
    shell: Entity<Shell>,
    field: TextField,
    admission: SessionAdmission,
    opening: SessionOpening,
    registry: Registry,
    observed: Observed,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        gpui::div().child(self.field.element(&Tokens::of(Scheme::Dark), false, cx))
    }
}
fn new_probe(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    let shell = cx.new(|cx| Shell::without_io(window, cx));
    let observed: Observed = Rc::default();
    let mut epochs = ConnectionEpochs::default();
    let original = connection(&mut epochs, NodeId::random());
    let registry = Registry(WorkspaceId::random());
    let opening =
        SessionOpening::resolve(&registry, original.clone(), SessionId::random()).unwrap();
    let admission = shell.update(cx, |shell, cx| {
        shell
            .admit_session_open(
                &opening,
                &registry,
                |context, window, cx| {
                    assert_eq!(context.resident.address.node, original.node());
                    let observed = Rc::clone(&observed);
                    let session = SessionView::new_scoped(
                        context.clone(),
                        move |shell, current, text, _, cx| {
                            shell
                                .areas
                                .session_residents
                                .resources_mut(current)
                                .unwrap()
                                .session
                                .resume_error = Some(text);
                            observed.borrow_mut().push(current.clone());
                            cx.notify();
                        },
                        window,
                        cx,
                    );
                    SessionResources::new(
                        session,
                        ConversationArea::with_display(ConversationDisplay::CollapseTools, cx),
                    )
                },
                |_, _| panic!("first admission retired"),
                window,
                cx,
            )
            .unwrap()
    });
    assert_eq!(admission.connection, original);
    let field = shell
        .read(cx)
        .areas
        .session_residents
        .resources(&admission.context)
        .unwrap()
        .session
        .composer
        .clone();
    field.set_text(" admitted input ", window, cx);
    Probe {
        shell,
        field,
        admission,
        opening,
        registry,
        observed,
    }
}
fn submit_and_check(probe: &Entity<Probe>, window: &mut gpui::VisualTestContext) {
    probe.update_in(window, |probe, window, cx| {
        window.activate_window();
        probe.field.focus(window, cx);
    });
    window.simulate_keystrokes("enter");
    window.run_until_parked();
    probe.read_with(window, |probe, cx| {
        assert_eq!(
            &*probe.observed.borrow(),
            &[probe.admission.context.clone()]
        );
        let shell = probe.shell.read(cx);
        assert_ne!(shell.areas.session.composer.id(), probe.field.id());
        let resources = shell
            .areas
            .session_residents
            .resources(&probe.admission.context)
            .unwrap();
        assert_eq!(resources.session.composer.id(), probe.field.id());
        assert_eq!(
            resources.session.resume_error.as_deref(),
            Some(" admitted input ")
        );
        assert_eq!(probe.field.text(cx), " admitted input ");
    });
}

#[gpui::test]
fn real_minters_scoped_factory_keeps_field_on_readmission_and_refuses_hidden_input(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, window) = cx.add_window_view(new_probe);
    window.run_until_parked();
    submit_and_check(&probe, window);
    probe.update_in(window, |probe, window, cx| {
        let again = probe.shell.update(cx, |shell, cx| {
            let next = shell
                .admit_session_open(
                    &probe.opening,
                    &probe.registry,
                    |_, _, _| panic!("existing actual fields must not be rebuilt"),
                    |_, notice| assert_eq!(notice.binding(), probe.admission.binding),
                    window,
                    cx,
                )
                .unwrap();
            let resources = shell
                .areas
                .session_residents
                .resources(&next.context)
                .unwrap();
            assert_eq!(resources.session.composer.id(), probe.field.id());
            assert_eq!(resources.session.composer.text(cx), " admitted input ");
            next
        });
        assert_eq!(again.context.resident, probe.admission.context.resident);
        assert_ne!(again.binding, probe.admission.binding);
        probe.admission = again;
        probe.shell.update(cx, |shell, _| {
            let context = &probe.admission.context;
            shell
                .areas
                .session_residents
                .present(
                    context.resident,
                    &context.claim,
                    context.pane,
                    Visibility::RetainedHidden,
                )
                .unwrap();
        });
    });
    // Retain platform focus deliberately: admission must refuse the hidden event.
    window.simulate_keystrokes("enter");
    window.run_until_parked();
    probe.read_with(window, |probe, cx| {
        assert_eq!(probe.observed.borrow().len(), 1);
        assert_eq!(probe.field.text(cx), " admitted input ");
    });
}

#[gpui::test]
fn full_retirement_after_sealed_ack_revokes_installed_lifecycle_hydration_with_empty_tickets(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, window) = cx.add_window_view(new_probe);
    probe.update_in(window, |probe, _, cx| {
        probe.shell.update(cx, |shell, cx| {
            let residents = &mut shell.areas.session_residents;
            hydrate_lifecycle(residents, &probe.admission);
            let binding = probe.admission.binding;
            // Pure stop protocol is driven directly here. This does not model
            // the future real Lifecycle ACK-held owner or its delivery guard.
            let ticket = residents
                .admission
                .admit_stop(binding, StopAction::Fresh)
                .unwrap();
            let StopSettlement::WindowHeld(receipt) = residents
                .admission
                .settle_stop(binding, ticket, StopCompletion::Acknowledged)
                .unwrap()
            else {
                panic!("ACK must hold")
            };
            let _seal = residents.admission.end_follow(binding).unwrap();
            residents
                .retire_follow(binding, |resources, notice| {
                    assert!(notice.operations().is_empty());
                    assert_eq!(
                        resources.lifecycle.readiness(),
                        OwnerReadiness::Hold(HoldReason::Loading)
                    );
                    assert_eq!(resources.session.composer.id(), probe.field.id());
                    assert_eq!(resources.session.composer.text(cx), " admitted input ");
                })
                .unwrap();
            assert!(
                residents
                    .admission
                    .validate_stop_window(binding, receipt)
                    .is_err()
            );
            assert!(residents.receive(binding, |_| panic!("retired")).is_err());
        });
    });
}
