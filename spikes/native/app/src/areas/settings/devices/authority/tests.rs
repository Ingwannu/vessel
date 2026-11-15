//! The real area callbacks use the same admitted facts as device dispatch.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::devices::Answer;
use vessel_app_view::devices::{Listing, Revoking};
use vessel_proto::{DeviceId, DeviceInfo, DeviceSelf, Scope};

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
#[cfg(not(target_arch = "wasm32"))]
mod actor_cases;
