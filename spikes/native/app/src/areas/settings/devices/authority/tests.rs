//! The real area callbacks use the same admitted facts as device dispatch.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::{
    Command, Link,
    devices::{Answer, Outcome},
};
use vessel_app_view::{
    connection::{ConnectionEpochs, MachineKey},
    devices::{Listing, Target},
};
use vessel_proto::{DeviceId, DeviceInfo, DeviceSelf, NodeId, Scope};

const COMMAND_CAPACITY: usize = 8;
fn device(id: DeviceId) -> DeviceInfo {
    DeviceInfo {
        id,
        label: "fixture".into(),
        created_at: chrono::DateTime::UNIX_EPOCH,
        last_seen: None,
        scopes: vec![Scope::Admin],
        revoked_at: None,
    }
}

fn prepare(
    shell: &mut Shell,
    context: &vessel_app_view::connection::ConnectionContext,
    window: &vessel_app_kit::ui::Window,
) {
    shell.route = crate::routes::Route::Settings;
    shell.areas.settings.section = vessel_app_view::settings::Section::Devices;
    shell.areas.settings.pushed = true;
    let mut machine =
        vessel_app_view::Machine::unreachable("fixture".into(), "session read refused");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines = vec![machine];
    shell.settle_devices(window);
}
fn reported(
    shell: &mut Shell,
    context: &vessel_app_view::connection::ConnectionContext,
    ticket: vessel_app_view::devices::Ticket,
    id: DeviceId,
    scope: Scope,
    cx: &mut Context<'_, Shell>,
) {
    shell.devices_answer(
        Answer {
            context: context.clone(),
            outcome: Outcome::Authority(
                ticket,
                Ok(DeviceSelf {
                    device: Some(id),
                    scopes: vec![scope],
                }),
            ),
        },
        cx,
    );
}

fn refuse_management(
    shell: &mut Shell,
    context: &vessel_app_view::connection::ConnectionContext,
    id: DeviceId,
    old_intent: vessel_app_view::devices::RevokeIntent,
    commands: &mut tokio::sync::mpsc::Receiver<Command>,
    window: &mut vessel_app_kit::ui::Window,
    cx: &mut Context<'_, Shell>,
) {
    assert!(!shell.areas.settings.devices.page.authority().may_manage());
    assert!(commands.try_recv().is_err());
    shell.devices_choose_revoke(context, id, window, cx);
    let request = shell
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
            .confirm_revoke(context, old_intent, request),
        None
    );
    for ask in [
        Ask::List,
        Ask::Invite { label: None },
        Ask::Revoke { device: id },
    ] {
        assert_eq!(
            shell.devices_send(context.clone(), request, ask),
            Err(vessel_app_view::devices::Problem::AccessNotConfirmed)
        );
    }
    shell.devices_list();
    assert!(
        commands.try_recv().is_err(),
        "a removed management grant creates no device-management request"
    );
    assert!(shell.areas.settings.devices.page.revoke_intent().is_none());
    assert!(matches!(
        shell.areas.settings.devices.page.target,
        Target::Online(_)
    ));
}

#[gpui::test]
fn reported_admin_without_read_opens_management_and_refresh_removes_old_intent(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        vessel_app_kit::init(
            cx,
            vessel_app_kit::SchemeChoice::Fixed(vessel_app_kit::Scheme::Dark),
        )
        .unwrap()
    });
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, &context, window);
        let Command::Devices {
            context: original,
            ticket,
            ask: Ask::Authority,
        } = commands.try_recv().unwrap()
        else {
            panic!("check authority before listing devices");
        };
        assert_eq!(original, context);
        assert!(commands.try_recv().is_err());
        let id = DeviceId::random();
        reported(shell, &context, ticket, id, Scope::Admin, cx);
        let Command::Devices {
            context: original,
            ticket: list,
            ask: Ask::List,
        } = commands.try_recv().unwrap()
        else {
            panic!("Admin without Read may list devices");
        };
        assert_eq!(original, context);
        assert_eq!(
            shell.areas.settings.devices.page.authority().device(),
            Some(id)
        );
        shell.devices_answer(
            Answer {
                context: context.clone(),
                outcome: Outcome::Listed(list, Ok(vec![device(id)])),
            },
            cx,
        );
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
        let Command::Devices {
            ticket: refresh,
            ask: Ask::Authority,
            ..
        } = commands.try_recv().unwrap()
        else {
            panic!("explicit refresh checks authority again");
        };
        reported(shell, &context, refresh, id, Scope::Read, cx);
        refuse_management(shell, &context, id, old_intent, &mut commands, window, cx);
    });
}
