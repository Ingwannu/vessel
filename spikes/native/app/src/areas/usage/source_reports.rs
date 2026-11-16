//! One generic card renderer consumes canonical node report values and independent current status.
use super::rows::{Density, notice};
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens,
    theme::spacing,
    ui::{AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, div, px},
};
use vessel_app_view::usage::{source_failure, source_problem};
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
        .gap(px(spacing::S3));
    if reports.policy == ExternalLookup::Off {
        return body
            .child(super::read_states::off(shell, tokens, cx))
            .into_any_element();
    }
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
        body = body.child(super::read_states::muted_card(text.plan_empty(), tokens));
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
    // Per-source consent is Vessel's (ADR 0042): it follows Paseo's cards.
    body.child(super::source_controls::render(
        shell, reports, density, tokens, cx,
    ))
    .into_any_element()
}
