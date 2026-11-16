//! Sidebar summaries use the one navigation preference and only available quota values.
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    Tokens, Type, TypeExt as _,
    agent_mark::{Size, Space},
    theme::spacing,
    usage_meter::{self, Appearance, Share},
};
use vessel_app_link::{Command, usage::Ask};
use vessel_app_view::{
    connection::ConnectionContext,
    sidebar::navigation::{Builtin, Visibility},
    usage::{
        Inventory, InventorySurface, MIN_SUMMARY_METER_WIDTH, SUMMARY_METER_GAP, SummaryAccount,
        SummaryLayout, SummaryMeasurements, SummaryWidth, SummaryWindowCount, pinned_windows,
        summary_layout,
    },
};
use vessel_proto::{limits::LimitKey, usage_reports::UsageSourceId};

pub(super) fn visible(shell: &Shell) -> bool {
    shell
        .prefs
        .settings
        .sidebar_navigation
        .visibility(Builtin::Usage)
        == Visibility::Shown
}
pub(super) fn selected(shell: &Shell) -> Option<ConnectionContext> {
    let hosts = super::modal::hosts(shell);
    let active = shell
        .areas
        .session
        .open
        .as_ref()
        .and_then(|open| {
            shell
                .fleet
                .machines
                .iter()
                .find(|machine| machine.name == open.machine)
        })
        .and_then(|machine| machine.node);
    let node = vessel_app_view::usage::HostChoice {
        picked: shell.prefs.settings.usage.host,
        active,
        hosts: &hosts,
    }
    .sidebar()?;
    shell
        .fleet
        .machines
        .iter()
        .find(|machine| machine.node == Some(node) && machine.connection_context().is_some())
        .and_then(vessel_app_view::Machine::connection_context)
}
impl Shell {
    pub(super) fn settle_summary_usage(&mut self) {
        let selected = visible(self).then(|| selected(self)).flatten();
        let state = &mut self.areas.usage;
        if selected.as_ref() == state.summary.selection() {
            return;
        }
        state.summary.close();
        if let Some(machine) = selected {
            if state
                .policy_pending
                .as_ref()
                .is_some_and(|pending| pending.machine == machine)
                || state
                    .source_pending
                    .as_ref()
                    .is_some_and(|pending| pending.machine == machine)
            {
                return;
            }
            if let Some(request) = state.summary.request(machine) {
                self.command(Command::Usage(Ask::Accounts {
                    surface: InventorySurface::Summary,
                    request,
                    refresh: vessel_app_view::usage::InventoryRefresh::Cached,
                }));
            }
        }
    }
    pub(super) fn refresh_usage_summary(&mut self, machine: &ConnectionContext) {
        let state = &mut self.areas.usage;
        if state.summary.selection() != Some(machine) {
            return;
        }
        if let Some(request) = state.summary.request(machine.clone()) {
            self.command(Command::Usage(Ask::Accounts {
                surface: InventorySurface::Summary,
                request,
                refresh: vessel_app_view::usage::InventoryRefresh::Cached,
            }));
        }
    }
}
/// Shared producer; S2 owns its visibility slot and the fixed icon callback.
pub(crate) fn sidebar_summary(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !visible(shell) {
        return None;
    }
    let inventory = shell.areas.usage.summary.phase().observation()?;
    let reports = candidates(inventory, shell.words);
    let pinned = pinned_windows(
        &reports,
        &shell.prefs.settings.usage.pins,
        shell.prefs.settings.usage.display,
    );
    if pinned.is_empty() {
        return None;
    }
    let available = shell
        .areas
        .usage
        .summary_place
        .get()
        .and_then(|bounds| SummaryWidth::measured(f32::from(bounds.size.width)));
    let labels = measure(&pinned, SummaryLayout::Labels, window);
    let percents = measure(&pinned, SummaryLayout::Percents, window);
    let layout = summary_layout(SummaryMeasurements {
        available,
        labels: SummaryWidth::measured(labels),
        percents: SummaryWidth::measured(percents),
        windows: SummaryWindowCount::of(&pinned),
    });
    let mut row = div()
        .id("sidebar-usage-summary")
        .w_full()
        .flex_none()
        .py(px(spacing::S1))
        .min_w_0()
        .relative()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .overflow_hidden()
        .cursor_pointer()
        .child(measure_probe(shell, cx))
        .on_click(cx.listener(|shell, _, window, cx| shell.open_usage(window, cx)));
    let mut previous = None;
    for pinned in pinned {
        if previous != Some(pinned.group) {
            row = row.child(vessel_app_kit::agent_mark::agent_mark(
                crate::agent_mark::artwork(vessel_app_view::usage::source_mark(
                    inventory,
                    pinned.source,
                )),
                Size::Sixteen,
                Space::Compact,
                tokens,
            ));
            previous = Some(pinned.group);
        }
        let label = match layout {
            SummaryLayout::Percents => pinned.percent.text(),
            SummaryLayout::Meters | SummaryLayout::Labels => format!(
                "{} {}",
                pinned.percent.text(),
                pinned
                    .window
                    .short_label
                    .as_deref()
                    .unwrap_or(&pinned.window.label)
            ),
        };
        row = row.child(
            div()
                .flex()
                .items_center()
                .gap(px(SUMMARY_METER_GAP))
                .flex_none()
                .when(layout == SummaryLayout::Meters, |row| {
                    row.child(div().w(px(MIN_SUMMARY_METER_WIDTH)).child(usage_meter::bar(
                        Share::percent(pinned.percent.whole()),
                        Appearance::Quiet,
                        tokens,
                    )))
                })
                .child(
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(label),
                ),
        );
    }
    Some(row.into_any_element())
}
#[expect(
    clippy::float_arithmetic,
    reason = "Shaped summary text and gaps are layout geometry, never token or monetary arithmetic"
)]
fn measure(
    pinned: &[vessel_app_view::usage::PinnedWindow<'_, UsageSourceId, LimitKey>],
    layout: SummaryLayout,
    window: &Window,
) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for pin in pinned {
        if previous != Some(pin.group) {
            width += Size::Sixteen.points() + spacing::S2;
            previous = Some(pin.group);
        }
        let label = match layout {
            SummaryLayout::Percents => pin.percent.text(),
            SummaryLayout::Meters | SummaryLayout::Labels => format!(
                "{} {}",
                pin.percent.text(),
                pin.window
                    .short_label
                    .as_deref()
                    .unwrap_or(&pin.window.label)
            ),
        };
        width += vessel_app_kit::fonts::measure(window, &label, Type::Meta) + spacing::S2;
    }
    width
}
pub(super) fn candidates(
    inventory: &Inventory,
    words: &'static vessel_app_view::localization::Translator,
) -> Vec<SummaryAccount<UsageSourceId, LimitKey>> {
    vessel_app_view::usage::summary_candidates(inventory, words)
}

fn measure_probe(shell: &Shell, cx: &Context<'_, Shell>) -> AnyElement {
    let place = std::rc::Rc::clone(&shell.areas.usage.summary_place);
    let this = cx.entity().downgrade();
    gpui::canvas(
        move |bounds, _, cx| {
            let changed = place
                .get()
                .is_none_or(|old| old.size.width != bounds.size.width);
            place.set(Some(bounds));
            if changed {
                let _gone = this.update(cx, |_, cx| cx.notify());
            }
        },
        |_, (), _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}
