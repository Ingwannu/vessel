//! Genuine framed Answers enter the actual area handler; no guard is fabricated.
use super::*;
use vessel_app_link::{Event, testing::Scenario};
use vessel_app_view::machine_book::Name;
use vessel_proto::{Call, Outcome as WireOutcome, Reply};

fn answer(scene: &mut Scenario, shell: &Shell) -> Answer {
    match scene.event(shell.fixture_link()).unwrap() {
        Event::Devices(answer) => answer,
        _other => panic!("expected a Devices response"),
    }
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
