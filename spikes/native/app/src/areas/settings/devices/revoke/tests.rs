//! Exercise the actual screen dispatch in one update, with no intervening
//! frame. The detached bounded link records commands without any node/I/O.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::{
    Command, Link,
    devices::{Answer, Outcome},
};
use vessel_app_view::devices::{ConnectionEpochs, Inviting, Listing, MachineKey, Target};
use vessel_proto::{DeviceInfo, NodeId};

fn device() -> DeviceInfo {
    DeviceInfo {
        id: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        label: "Same public device id, different physical machine".into(),
        created_at: chrono::DateTime::UNIX_EPOCH,
        last_seen: None,
        revoked_at: None,
        scopes: vec![vessel_proto::Scope::Admin],
    }
}

mod dismissal;
const COMMAND_CAPACITY: usize = 8;

fn install(shell: &mut Shell, context: &vessel_app_view::connection::ConnectionContext) {
    let mut machine = vessel_app_view::Machine::unreachable(
        context.machine().as_str().into(),
        "session read refused",
    );
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines = vec![machine];
    shell.devices_fleet();
}

fn grant(
    page: &mut vessel_app_view::devices::Page,
    context: &vessel_app_view::connection::ConnectionContext,
) {
    let ticket = page.tickets.next_ticket().unwrap();
    assert!(page.checking_authority(context, ticket));
    assert!(page.authority_answer(
        context,
        ticket,
        Ok(vessel_proto::DeviceSelf {
            device: None,
            scopes: vec![vessel_proto::Scope::Admin],
        })
    ));
}

#[gpui::test]
fn connection_events_refuse_old_controls_and_answers_before_another_frame(cx: &mut TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(
            cx,
            vessel_app_kit::SchemeChoice::Fixed(vessel_app_kit::Scheme::Dark),
        )
        .unwrap();
    });
    let mut epochs = ConnectionEpochs::default();
    let first = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let next = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        shell.route = crate::routes::Route::Settings;
        shell.areas.settings.section = vessel_app_view::settings::Section::Devices;
        shell.areas.settings.pushed = true;
        shell.fleet.machines = vec![vessel_app_view::Machine::unreachable(
            "fixture".into(),
            "session read refused",
        )];
        shell
            .areas
            .settings
            .devices
            .page
            .target(Target::Online(first.machine().clone()));
        install(shell, &first);
        grant(&mut shell.areas.settings.devices.page, &first);
        shell.areas.settings.devices.page.listing = Listing::Ready(vec![device()]);
        shell.devices_choose_revoke(&first, device().id, window, cx);
        let old_intent = shell.areas.settings.devices.page.revoke_intent().unwrap();
        install(shell, &next);
        assert!(matches!(
            shell.areas.settings.devices.page.revoking,
            Revoking::Empty
        ));
        assert!(
            shell
                .areas
                .settings
                .devices
                .page
                .inspected_device()
                .is_none()
        );
        shell.devices_choose_revoke(&first, device().id, window, cx);
        assert!(matches!(
            shell.areas.settings.devices.page.revoking,
            Revoking::Empty
        ));
        // Reconciliation may start the new admission's normal read. It must
        // never turn the old row press into a write on that admission.
        let Command::Devices {
            context,
            ticket: request,
            ask: Ask::Authority,
        } = commands.try_recv().unwrap()
        else {
            panic!("the replacement starts one read, never a stale write");
        };
        assert_eq!(context, next);
        assert!(commands.try_recv().is_err());
        refuse_old_answers(shell, cx, &first, request);
        grant(&mut shell.areas.settings.devices.page, &next);
        shell.areas.settings.devices.page.listing = Listing::Ready(vec![device()]);
        shell.devices_choose_revoke(&next, device().id, window, cx);
        let intent = shell.areas.settings.devices.page.revoke_intent().unwrap();
        shell.devices_revoke(&first, old_intent, window);
        shell.devices_revoke(&next, old_intent, window);
        assert!(commands.try_recv().is_err());
        shell.devices_revoke(&next, intent, window);
        assert!(matches!(commands.try_recv().unwrap(), Command::Devices {
            context, ask: Ask::Revoke { device: id }, ..
        } if context == next && id == device().id));
        shell.devices_revoke(&next, intent, window);
        assert!(commands.try_recv().is_err());
        assert!(!shell.devices_retired(&first.tag()));
        assert!(matches!(
            shell.areas.settings.devices.page.revoking,
            Revoking::Sending { .. }
        ));
        assert!(shell.devices_retired(&next.tag()));
        assert!(matches!(
            shell.areas.settings.devices.page.revoking,
            Revoking::Empty
        ));
        assert!(matches!(
            shell.areas.settings.devices.page.inviting,
            Inviting::Empty
        ));
    });
}

fn refuse_old_answers(
    shell: &mut Shell,
    cx: &mut Context<'_, Shell>,
    first: &vessel_app_view::connection::ConnectionContext,
    request: vessel_app_view::devices::Ticket,
) {
    shell.areas.settings.devices.page.listing = Listing::Loading(request);
    shell.devices_answer(
        Answer {
            context: first.clone(),
            outcome: Outcome::Authority(
                request,
                Ok(vessel_proto::DeviceSelf {
                    device: Some(device().id),
                    scopes: vec![vessel_proto::Scope::Admin],
                }),
            ),
        },
        cx,
    );
    assert!(matches!(shell.areas.settings.devices.page.authority(),
        vessel_app_view::devices::Authority::Loading(t) if *t == request));
    let invite = shell
        .areas
        .settings
        .devices
        .page
        .tickets
        .next_ticket()
        .unwrap();
    shell.areas.settings.devices.page.inviting = Inviting::Loading(invite);
    shell.devices_answer(
        Answer {
            context: first.clone(),
            outcome: Outcome::Listed(request, Ok(vec![device()])),
        },
        cx,
    );
    shell.devices_answer(
        Answer {
            context: first.clone(),
            outcome: Outcome::Invited(request, Err(vessel_app_view::devices::Problem::Offline)),
        },
        cx,
    );
    assert!(matches!(
        shell.areas.settings.devices.page.listing,
        Listing::Loading(_)
    ));
    assert!(matches!(
        shell.areas.settings.devices.page.inviting,
        Inviting::Loading(_)
    ));
}
