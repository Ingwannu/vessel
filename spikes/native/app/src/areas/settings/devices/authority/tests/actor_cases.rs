//! Genuine framed Answers enter the actual area handler; no guard is fabricated.
use super::*;
use vessel_app_link::{Event, testing::Scenario};
use vessel_app_view::machine_book::Name;
use vessel_proto::{Call, Outcome as WireOutcome, Reply};

pub(super) fn answer(scene: &mut Scenario, shell: &Shell) -> Answer {
    // Retirement Fleet notices may be held behind an already queued response.
    // Consume only bounded fixture events; never install them as authority.
    const MAX_EVENTS: usize = 32;
    for _ in 0..MAX_EVENTS {
        if let Event::Devices(answer) = scene.event(shell.fixture_link()).unwrap() {
            return answer;
        }
    }
    panic!("expected a bounded Devices response");
}

#[gpui::test]
fn actual_actor_opens_management_after_the_reported_admin_grant(cx: &mut TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(
            cx,
            vessel_app_kit::SchemeChoice::Fixed(vessel_app_kit::Scheme::Dark),
        )
        .unwrap();
    });
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, &context, window);
        assert!(!shell.areas.settings.devices.page.authority().may_manage());
        assert!(matches!(
            scene.request(shell.fixture_link()).unwrap(),
            Call::GetDeviceSelf(_)
        ));
        let id = DeviceId::random();
        scene
            .reply(
                shell.fixture_link(),
                WireOutcome::Ok(Reply::GetDeviceSelf(DeviceSelf {
                    device: Some(id),
                    scopes: vec![Scope::Admin],
                })),
            )
            .unwrap();
        let authority = answer(&mut scene, shell);
        shell.devices_answer(authority, cx);
        assert!(shell.areas.settings.devices.page.authority().may_manage());
        assert_eq!(
            shell.areas.settings.devices.page.authority().device(),
            Some(id)
        );
        assert!(matches!(
            scene.request(shell.fixture_link()).unwrap(),
            Call::ListDevices(_)
        ));
        scene
            .reply(
                shell.fixture_link(),
                WireOutcome::Ok(Reply::ListDevices(vec![device(id)])),
            )
            .unwrap();
        let listed = answer(&mut scene, shell);
        shell.devices_answer(listed, cx);
        shell.devices_choose_revoke(&context, id, window, cx);
        assert!(shell.areas.settings.devices.page.revoke_intent().is_some());
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 2);
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_held_authority_reply_cannot_install_before_the_retired_fleet_frame(cx: &mut TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(
            cx,
            vessel_app_kit::SchemeChoice::Fixed(vessel_app_kit::Scheme::Dark),
        )
        .unwrap();
    });
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, &context, window);
        assert!(matches!(
            scene.request(shell.fixture_link()).unwrap(),
            Call::GetDeviceSelf(_)
        ));
        scene
            .reply(
                shell.fixture_link(),
                WireOutcome::Ok(Reply::GetDeviceSelf(DeviceSelf {
                    device: Some(DeviceId::random()),
                    scopes: vec![Scope::Admin],
                })),
            )
            .unwrap();
        let held = answer(&mut scene, shell);
        scene.retire(shell.fixture_link()).unwrap();
        // Deliberately do not install the actor's newer Fleet/retirement event.
        assert_eq!(
            shell.fleet.machines[0].connection_context(),
            Some(context.clone())
        );
        assert_eq!(shell.areas.settings.devices.page.context(), Some(&context));
        shell.devices_answer(held, cx);
        assert!(!shell.areas.settings.devices.page.authority().may_manage());
        assert!(!matches!(
            shell.areas.settings.devices.page.listing,
            Listing::Ready(_)
        ));
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 1);
        scene.close(shell.fixture_link()).unwrap();
    });
}

fn report(
    scene: &mut Scenario,
    shell: &mut Shell,
    id: DeviceId,
    scopes: Vec<Scope>,
    cx: &mut Context<'_, Shell>,
) {
    assert!(matches!(
        scene.request(shell.fixture_link()).unwrap(),
        Call::GetDeviceSelf(_)
    ));
    scene
        .reply(
            shell.fixture_link(),
            WireOutcome::Ok(Reply::GetDeviceSelf(DeviceSelf {
                device: Some(id),
                scopes,
            })),
        )
        .unwrap();
    let response = answer(scene, shell);
    shell.devices_answer(response, cx);
}
fn listed(scene: &mut Scenario, shell: &mut Shell, id: DeviceId, cx: &mut Context<'_, Shell>) {
    assert!(matches!(
        scene.request(shell.fixture_link()).unwrap(),
        Call::ListDevices(_)
    ));
    scene
        .reply(
            shell.fixture_link(),
            WireOutcome::Ok(Reply::ListDevices(vec![device(id)])),
        )
        .unwrap();
    let response = answer(scene, shell);
    shell.devices_answer(response, cx);
}
fn initialize(cx: &mut TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(
            cx,
            vessel_app_kit::SchemeChoice::Fixed(vessel_app_kit::Scheme::Dark),
        )
        .unwrap()
    });
}

#[gpui::test]
fn a_real_refresh_removes_admin_and_old_confirmation_without_session_read(cx: &mut TestAppContext) {
    initialize(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, &context, window);
        let id = DeviceId::random();
        report(&mut scene, shell, id, vec![Scope::Admin], cx);
        listed(&mut scene, shell, id, cx);
        shell.devices_choose_revoke(&context, id, window, cx);
        let old_intent = shell.areas.settings.devices.page.revoke_intent().unwrap();
        shell.devices_authority();
        assert!(matches!(
            shell.areas.settings.devices.page.listing,
            Listing::Empty
        ));
        assert!(matches!(
            shell.areas.settings.devices.page.revoking,
            Revoking::Empty
        ));
        report(&mut scene, shell, id, vec![Scope::Read], cx);
        shell.devices_choose_revoke(&context, id, window, cx);
        let ticket = shell
            .areas
            .settings
            .devices
            .page
            .tickets
            .next_ticket()
            .unwrap();
        assert_eq!(
            shell
                .areas
                .settings
                .devices
                .page
                .confirm_revoke(&context, old_intent, ticket),
            None
        );
        for ask in [
            Ask::List,
            Ask::Invite { label: None },
            Ask::Revoke { device: id },
        ] {
            assert_eq!(
                shell.devices_send(context.clone(), ticket, ask),
                Err(vessel_app_view::devices::Problem::AccessNotConfirmed)
            );
        }
        shell.devices_list();
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 3);
        assert!(!shell.areas.settings.devices.page.authority().may_manage());
        assert_eq!(shell.areas.settings.devices.page.context(), Some(&context));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn retired_proof_clears_access_before_a_callback_or_frame_without_a_fleet_event(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, &context, window);
        let id = DeviceId::random();
        report(&mut scene, shell, id, vec![Scope::Admin], cx);
        listed(&mut scene, shell, id, cx);
        scene.retire(shell.fixture_link()).unwrap();
        assert_eq!(
            shell.fleet.machines[0].connection_context(),
            Some(context.clone())
        );
        assert!(!shell.devices_press(&context, window));
        shell.devices_choose_revoke(&context, id, window, cx);
        assert!(shell.areas.settings.devices.page.context().is_none());
        assert!(!shell.areas.settings.devices.page.authority().may_manage());
        assert!(matches!(
            shell.areas.settings.devices.page.listing,
            Listing::Empty
        ));
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 2);
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_queued_revoke_success_after_retirement_is_unconfirmed_and_never_replayed(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, &context, window);
        let id = DeviceId::random();
        report(&mut scene, shell, id, vec![Scope::Admin], cx);
        listed(&mut scene, shell, id, cx);
        shell.areas.settings.devices.name.set_text("Retained invitation name", window, cx);
        shell.devices_choose_revoke(&context, id, window, cx);
        let page = &mut shell.areas.settings.devices.page;
        let ticket = page.tickets.next_ticket().unwrap();
        let intent = page.revoke_intent().unwrap();
        let selected = page.confirm_revoke(&context, intent, ticket).unwrap();
        shell.devices_send(context.clone(), ticket, Ask::Revoke { device: selected }).unwrap();
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::RevokeDevice(ask) if ask.device == id));
        scene.reply(shell.fixture_link(), WireOutcome::Ok(Reply::RevokeDevice(vessel_proto::Ack {}))).unwrap();
        let held = answer(&mut scene, shell);
        scene.retire(shell.fixture_link()).unwrap();
        shell.devices_answer(held, cx);
        assert_eq!(shell.areas.settings.devices.page.unconfirmed(&shell.fleet.machines[0]),
            Some(vessel_app_view::devices::Unconfirmed::Revocation));
        assert!(matches!(shell.areas.settings.devices.page.revoking, Revoking::Empty));
        assert_eq!(shell.areas.settings.devices.name.text(cx), "Retained invitation name");
        assert!(!shell.areas.settings.devices.page.authority().may_manage());
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 3);
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_held_old_list_cannot_install_or_retire_a_same_node_successors_pending_write(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, &context, window);
        let id = DeviceId::random();
        report(&mut scene, shell, id, vec![Scope::Admin], cx);
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::ListDevices(_)));
        scene.reply(shell.fixture_link(), WireOutcome::Ok(Reply::ListDevices(vec![device(id)]))).unwrap();
        let held = answer(&mut scene, shell);
        let current = scene.reconnect(shell.fixture_link(), vessel_app_link::testing::Reconnect::SameNode).unwrap();
        assert_eq!(context.node(), current.node());
        assert_ne!(context.epoch(), current.epoch());
        prepare(shell, &current, window);
        report(&mut scene, shell, id, vec![Scope::Admin], cx);
        listed(&mut scene, shell, id, cx);
        shell.devices_choose_revoke(&current, id, window, cx);
        let page = &mut shell.areas.settings.devices.page;
        let intent = page.revoke_intent().unwrap();
        let ticket = page.tickets.next_ticket().unwrap();
        let selected = page.confirm_revoke(&current, intent, ticket).unwrap();
        shell.devices_send(current.clone(), ticket, Ask::Revoke { device: selected }).unwrap();
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::RevokeDevice(_)));
        shell.devices_answer(held, cx);
        assert_eq!(shell.areas.settings.devices.page.context(), Some(&current));
        assert!(matches!(shell.areas.settings.devices.page.revoking, Revoking::Sending { ticket: pending, .. } if pending == ticket));
        assert_eq!(shell.areas.settings.devices.page.unconfirmed(&shell.fleet.machines[0]), None);
        scene.reply(shell.fixture_link(), WireOutcome::Ok(Reply::RevokeDevice(vessel_proto::Ack {}))).unwrap();
        let response = answer(&mut scene, shell);
        shell.devices_answer(response, cx);
        assert!(matches!(shell.areas.settings.devices.page.revoking, Revoking::Empty));
        // The successful write asks for exactly one fresh list, never replays.
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::ListDevices(_)));
        scene.reply(shell.fixture_link(), WireOutcome::Ok(Reply::ListDevices(Vec::new()))).unwrap();
        let response = answer(&mut scene, shell);
        shell.devices_answer(response, cx);
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 4);
        scene.close(shell.fixture_link()).unwrap();
    });
}
