//! One shell clock rereads cached facts only; explicit refresh is the only external lookup.
use crate::shell::Shell;
use std::time::Duration;
use vessel_app_kit::ui::Context;
use vessel_app_link::{Command, usage::Ask};
use vessel_app_view::{
    devices::Target,
    usage::{InventorySurface, Phase},
};
use vessel_proto::limits::LimitRefresh;
const CACHE_READ_EVERY: Duration = Duration::from_secs(60);
pub(super) fn start(cx: &mut Context<'_, Shell>) {
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor().timer(CACHE_READ_EVERY).await;
            if this
                .update(cx, |shell, cx| {
                    let visible = matches!(shell.areas.usage.target, Target::Online(_))
                        || shell.areas.usage.summary.selection().is_some()
                        || shell.areas.usage.session.selection().is_some();
                    if visible {
                        shell.usage_cached_tick();
                        cx.notify();
                    }
                })
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}
impl Shell {
    fn usage_cached_tick(&mut self) {
        let state = &self.areas.usage;
        if state.policy_pending.is_some() || state.source_pending.is_some() {
            return;
        }
        if matches!(state.target, Target::Online(_))
            && !matches!(state.inventory.phase(), Phase::Loading { .. })
        {
            self.usage_accounts(vessel_app_view::usage::InventoryRefresh::Cached);
        }
        let state = &mut self.areas.usage;
        if !matches!(state.summary.phase(), Phase::Loading { .. })
            && let Some(machine) = state.summary.selection().cloned()
            && let Some(request) = state.summary.request(machine)
        {
            self.command(Command::Usage(Ask::Accounts {
                surface: InventorySurface::Summary,
                request,
                refresh: vessel_app_view::usage::InventoryRefresh::Cached,
            }));
        }
        let reader = &mut self.areas.usage.session;
        if !matches!(reader.phase(), Phase::Loading { .. })
            && let Some(selection) = reader.selection().cloned()
            && let Some(request) = reader.request(selection)
        {
            self.command(Command::Usage(Ask::Session {
                request,
                refresh: LimitRefresh::Cached,
            }));
        }
    }
}
