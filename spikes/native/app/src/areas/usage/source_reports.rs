//! One generic card renderer consumes canonical node report values and independent current status.
use super::rows::{Density, message, notice};
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens, outline_button, settings,
    theme::spacing,
    ui::{
        AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
        StatefulInteractiveElement as _, Styled as _, div, px,
    },
};
use vessel_app_view::usage::{Phase, source_failure, source_problem, source_view, timestamp};
use vessel_proto::{
    limits::ExternalLookup,
    usage_reports::{
        SourceReportEntry, UsageAttemptFailure, UsageCompleteness, UsageReports,
        UsageSourceAvailability,
    },
};
pub(super) fn render(
    shell: &Shell,
    reports: &UsageReports,
    density: Density,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let mut body = div()
        .min_w_0()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .child(super::source_controls::render(
            shell, reports, density, tokens, cx,
        ));
    if reports.policy == ExternalLookup::Off {
        return body
            .child(message(text.lookup_off(), tokens))
            .into_any_element();
    }
    let enabled = state.policy_pending.is_none()
        && state.source_pending.is_none()
        && !matches!(state.inventory.phase(), Phase::Loading { .. })
        && reports
            .sources
            .as_slice()
            .iter()
            .any(|source| source.availability == UsageSourceAvailability::Enabled);
    let machine = state.inventory.selection().cloned();
    body = body.child(
        outline_button(
            "usage-reports-refresh-all",
            text.refresh_all(),
            enabled,
            tokens,
        )
        .when(enabled, |button| {
            button.on_click(cx.listener(move |shell, _, _, cx| {
                if let Some(machine) = &machine {
                    shell.usage_refresh_all(machine.clone());
                    cx.notify();
                }
            }))
        }),
    );
    for failure in reports.source_failures.as_slice() {
        let reason = match &failure.failure {
            UsageAttemptFailure::Unavailable { problem } => source_problem(problem, shell.words),
            UsageAttemptFailure::Error { reason } => source_failure(*reason, shell.words),
        };
        let label = reports
            .sources
            .as_slice()
            .iter()
            .find(|source| source.id == failure.source)
            .map_or(text.source_reports(), |source| source.label.as_str());
        body = body.child(notice(
            format!("{label}: {reason}"),
            vessel_app_kit::alert::Variant::Warning,
            tokens,
        ));
    }
    if reports.reports.is_empty() {
        body = body.child(message(text.no_source_reports(), tokens));
    }
    for entry in reports.reports.as_slice() {
        if reports
            .sources
            .as_slice()
            .iter()
            .find(|source| &source.id == entry.source())
            .is_none_or(|source| source.availability == UsageSourceAvailability::Enabled)
        {
            body = body.child(card(shell, reports, entry, density, tokens, cx));
        }
    }
    body.into_any_element()
}
fn card(
    shell: &Shell,
    reports: &UsageReports,
    entry: &SourceReportEntry,
    density: Density,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let view = source_view(&entry.state, shell.words);
    let title = entry.account_label.as_ref().map_or_else(
        || entry.source_label.as_str().to_owned(),
        |account| format!("{} · {}", entry.source_label, account),
    );
    let mut body = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .flex()
                .items_center()
                .child(vessel_app_kit::agent_mark::agent_mark(
                    crate::agent_mark::artwork(vessel_app_view::usage::builtin_source_mark(
                        entry.builtin,
                    )),
                    vessel_app_kit::agent_mark::Size::Fourteen,
                    vessel_app_kit::agent_mark::Space::Row,
                    tokens,
                ))
                .child(message(text.account_disclosure(), tokens)),
        );
    if let Some(status) = &view.status {
        body = body.child(message(status.clone(), tokens));
    }
    if view.retained {
        body = body.child(message(text.retained(), tokens));
    }
    body = body.child(observed_content(
        shell, reports, entry, &view, density, tokens, cx,
    ));
    for (at, retry) in [(view.attempted_at, false), (view.retry_at, true)] {
        if let Some(at) = at {
            let at = timestamp(at, state.offset, shell.words);
            let label = if retry {
                text.retry_at(&at)
            } else {
                text.attempted_at(&at)
            };
            body = body.child(message(label.unwrap_or(at), tokens));
        }
    }
    let enabled = state.policy_pending.is_none()
        && state.source_pending.is_none()
        && !matches!(state.inventory.phase(), Phase::Loading { .. })
        && reports.sources.as_slice().iter().any(|source| {
            &source.id == entry.source() && source.availability == UsageSourceAvailability::Enabled
        });
    let machine = state.inventory.selection().cloned();
    let id = entry.id().clone();
    let refresh = outline_button(
        vessel_app_kit::ui::SharedString::from(format!("usage-report-refresh-{id}")),
        text.refresh(),
        enabled,
        tokens,
    )
    .when(enabled, |button| {
        button.on_click(cx.listener(move |shell, _, _, cx| {
            if let Some(machine) = &machine {
                shell.usage_refresh_report(machine.clone(), id.clone());
                cx.notify();
            }
        }))
    });
    settings::section_with_trailing(
        title,
        settings::card(tokens).child(body),
        Some(refresh.into_any_element()),
        tokens,
    )
}

fn observed_content(
    shell: &Shell,
    reports: &UsageReports,
    entry: &SourceReportEntry,
    view: &vessel_app_view::usage::SourceView<'_>,
    density: Density,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let mut body = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2));
    if let Some(observed) = view.observed {
        let report = &observed.report;
        if report.completeness == UsageCompleteness::Partial {
            body = body.child(notice(
                text.partial(),
                vessel_app_kit::alert::Variant::Warning,
                tokens,
            ));
        }
        if let Some(plan) = &report.plan {
            body = body.child(message(plan.as_str().to_owned(), tokens));
        }
        for window in report.windows.as_slice() {
            body = body.child(super::source_windows::render(
                shell,
                entry,
                window,
                view.retained,
                tokens,
                cx,
            ));
        }
        for balance in report.balances.as_slice() {
            body = body.child(super::source_balances::render(
                shell,
                balance,
                reports.cost_visibility,
                density,
                tokens,
            ));
        }
        for detail in report.details.as_slice() {
            if detail.access == vessel_proto::usage_reports::UsageDetailAccess::Read
                || reports.cost_visibility == vessel_proto::usage::CostVisibility::Permitted
            {
                body = body.child(message(
                    format!("{}: {}", detail.label, detail.value),
                    tokens,
                ));
            }
        }
        let at = timestamp(observed.at, state.offset, shell.words);
        body = body.child(message(text.refreshed_at(&at).unwrap_or(at), tokens));
    }
    body
}
