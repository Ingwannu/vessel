//! Actual bundled owners, without a transport guard or product caller fixture.
use super::*;
use std::time::Duration;
use vessel_app_view::residents::ResourceOwner;

fn hydrate_installed(residents: &mut SessionResidents, admission: &SessionAdmission) {
    residents
        .receive(admission.binding, |resources| {
            let snapshot = Snapshot::received(resources, admission).unwrap();
            resources.lifecycle.hydrate(snapshot).unwrap();
        })
        .unwrap();
}

#[gpui::test]
fn bundled_loading_blocks_table_ready_and_reattach_revokes_hydration(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let admission = admitted(shell, window, cx);
        let context = &admission.context;
        let residents = &mut shell.areas.session_residents;
        residents
            .resources_mut(context)
            .unwrap()
            .session
            .composer
            .set_text(" retained bundle ", window, cx);
        for owner in ResourceOwner::ALL {
            residents
                .readiness(context.resident, owner, OwnerReadiness::Ready)
                .unwrap();
        }
        residents
            .release_claim(context.resident, &context.claim, Duration::ZERO)
            .unwrap();
        assert!(matches!(
            residents.retire_clean(context.resident),
            Err(ResidentProblem::Held)
        ));
        hydrate_installed(residents, &admission);
        let mut callbacks = 0_usize;
        let fresh = residents
            .begin_follow(context.resident, |resources, notice| {
                callbacks = callbacks.saturating_add(1);
                assert_eq!(notice.binding(), admission.binding);
                assert_eq!(
                    resources.lifecycle.readiness(),
                    OwnerReadiness::Hold(HoldReason::Loading)
                );
            })
            .unwrap();
        assert_eq!(callbacks, 1);
        assert!(residents.receive(admission.binding, |_| ()).is_err());
        residents
            .receive(fresh, |resources| {
                assert_eq!(resources.session.composer.text(cx), " retained bundle ");
                assert_eq!(
                    resources.lifecycle.prepare(fresh, LifecycleAction::Fresh),
                    Err(Problem::Held)
                );
            })
            .unwrap();
        assert!(matches!(
            residents.retire_clean(context.resident),
            Err(ResidentProblem::Held)
        ));
    });
}

#[gpui::test]
fn bundled_retirement_preserves_exact_pending_lifecycle_and_input(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let admission = admitted(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        hydrate_installed(residents, &admission);
        let binding = admission.binding;
        let decision = residents
            .receive(binding, |resources| {
                resources
                    .session
                    .composer
                    .set_text(" pending input ", window, cx);
                resources.lifecycle.prepare(binding, LifecycleAction::Fresh)
            })
            .unwrap()
            .unwrap();
        let ticket = residents
            .admit_operation(binding, OperationKind::Lifecycle)
            .unwrap();
        residents
            .receive(binding, |resources| {
                resources.lifecycle.sent(decision, ticket)
            })
            .unwrap()
            .unwrap();
        residents
            .retire_follow(binding, |resources, _| {
                let pending = resources.lifecycle.uncertain().unwrap();
                assert_eq!(pending.binding, binding);
                assert_eq!(pending.ticket, ticket);
                assert_eq!(resources.session.composer.text(cx), " pending input ");
            })
            .unwrap();
        assert!(
            residents
                .settle(binding, OperationKind::Lifecycle, ticket, |_| panic!(
                    "retired"
                ))
                .is_err()
        );
        let resources = residents.resources(&admission.context).unwrap();
        assert_eq!(
            resources.lifecycle.readiness(),
            OwnerReadiness::Hold(HoldReason::UncertainEffect)
        );
        assert_eq!(resources.session.composer.text(cx), " pending input ");
    });
}
