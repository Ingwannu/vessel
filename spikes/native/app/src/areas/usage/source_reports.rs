//! One generic card renderer consumes canonical node report values and independent current status.
use super::rows::{Density, message, notice};
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens, outline_button,
    theme::spacing,
    ui::{
        AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
        StatefulInteractiveElement as _, Styled as _, div, px,
    },
};
use vessel_app_view::usage::{Phase, source_failure, source_problem};
use vessel_proto::{
    limits::ExternalLookup,
    usage_reports::{UsageAttemptFailure, UsageReports, UsageSourceAvailability},
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
    state
        .source_tips
        .prepare(state.inventory.observation_ticket(), reports);
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
            .child(super::read_states::off(shell, tokens, cx))
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
    let receipt = super::read_states::context(shell);
    body = body.child(
        outline_button(
            "usage-reports-refresh-all",
            text.refresh_all(),
            enabled,
            tokens,
        )
        .when(enabled, |button| {
            button.on_click(cx.listener(move |shell, _, _, cx| {
                if let Some(receipt) = &receipt
                    && super::read_states::current(shell, receipt)
                {
                    shell.usage_refresh_all(receipt.machine().clone());
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
            body = body.child(super::source_card::render(
                shell, reports, entry, density, tokens, cx,
            ));
        }
    }
    body.into_any_element()
}
