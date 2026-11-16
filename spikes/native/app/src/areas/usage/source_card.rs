//! Canonical report cards in Paseo's card frame (card.rs), with the shared
//! badge, dot, meter and tooltip controls.
use super::{
    card::{self, Footer, Head, Status},
    rows::Density,
};
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens,
    theme::spacing,
    ui::{
        AnyElement, Context, Div, IntoElement as _, ParentElement as _, SharedString, Styled as _,
        div, px,
    },
};
use vessel_app_view::usage::{
    Phase, source_freshness, source_login_errors, source_view, timestamp,
};
use vessel_proto::usage_reports::{
    SourceReportEntry, SourceReportState, UsageCompleteness, UsageReports, UsageSourceAvailability,
};
pub(super) fn render(
    shell: &Shell,
    reports: &UsageReports,
    entry: &SourceReportEntry,
    density: Density,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let compact = matches!(density, Density::Compact);
    let view = source_view(&entry.state, shell.words);
    let mut body = Vec::new();
    let login_errors = source_login_errors(&entry.state, shell.words);
    if login_errors.is_empty() {
        if let Some(status) = &view.status {
            body.push(card::quiet(status.clone().into(), tokens));
        }
    } else {
        for error in login_errors {
            body.push(card::error(error.into(), tokens));
        }
    }
    body.extend(observed_content(shell, reports, entry, &view, tokens, cx));
    for (at, retry) in [(view.attempted_at, false), (view.retry_at, true)] {
        if let Some(at) = at {
            let at = timestamp(at, state.offset, shell.words);
            let label = if retry {
                text.retry_at(&at)
            } else {
                text.attempted_at(&at)
            };
            body.push(card::quiet(label.unwrap_or(at).into(), tokens));
        }
    }
    if matches!(entry.state, SourceReportState::Failed { .. }) {
        body.push(card::error(text.refresh_failed().into(), tokens));
    }
    let availability = reports
        .sources
        .as_slice()
        .iter()
        .find(|source| &source.id == entry.source())
        .map(|source| source.availability);
    let freshness = source_freshness(&view, chrono::Utc::now(), shell.words);
    // Only a source this machine may read gets Refresh (Paseo has none for
    // a source that cannot report); consent and support are both required.
    let refresh = (availability == Some(UsageSourceAvailability::Enabled)).then(|| {
        let enabled = state.policy_pending.is_none()
            && state.source_pending.is_none()
            && !matches!(state.inventory.phase(), Phase::Loading { .. });
        let busy = matches!(state.inventory.phase(), Phase::Loading { .. })
            || matches!(entry.state, SourceReportState::Refreshing { .. });
        let receipt = super::read_states::context(shell);
        let id = entry.id().clone();
        let button = super::toolbar::refresh_button(
            SharedString::from(format!("usage-report-refresh-{id}")),
            text.refresh(),
            enabled,
            busy,
            compact,
            tokens,
        )
        .on_click(cx.listener(move |shell, _, _, cx| {
            if let Some(receipt) = &receipt
                && super::read_states::current(shell, receipt)
            {
                shell.usage_refresh_report(receipt.machine(), id.clone());
                cx.notify();
            }
        }));
        state
            .source_tips
            .on(button, entry, &freshness)
            .into_any_element()
    });
    let head = Head {
        mark: vessel_app_kit::agent_mark::agent_mark(
            crate::agent_mark::artwork(vessel_app_view::usage::builtin_source_mark(entry.builtin)),
            vessel_app_kit::agent_mark::Size::Fourteen,
            vessel_app_kit::agent_mark::Space::Row,
            tokens,
        )
        .into_any_element(),
        name: entry.source_label.as_str().to_owned().into(),
        plan: view
            .observed
            .and_then(|observed| observed.report.plan.as_ref())
            .map(|plan| plan.as_str().to_owned().into()),
        status: status(entry, availability),
        refresh,
    };
    let footer = Footer {
        account: entry
            .account_label
            .as_ref()
            .map(|account| account.as_str().to_owned().into()),
        freshness: compact.then(|| freshness.into()),
    };
    card::frame(head, body, footer, shell.words, tokens)
}

/// Paseo card.tsx `statusText`: Error for a failed attempt, Unavailable for
/// a source that cannot report; a current or refreshing report shows none.
fn status(
    entry: &SourceReportEntry,
    availability: Option<UsageSourceAvailability>,
) -> Option<Status> {
    match entry.state {
        SourceReportState::Failed { .. } => Some(Status::Error),
        SourceReportState::Unavailable { .. } => Some(Status::Unavailable),
        SourceReportState::Refreshing { .. }
        | SourceReportState::NotRequested
        | SourceReportState::Available { .. } => (availability
            == Some(UsageSourceAvailability::Unsupported))
        .then_some(Status::Unavailable),
    }
}

fn observed_content(
    shell: &Shell,
    reports: &UsageReports,
    entry: &SourceReportEntry,
    view: &vessel_app_view::usage::SourceView<'_>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let text = shell.words.usage();
    let mut body = Vec::new();
    let Some(observed) = view.observed else {
        return body;
    };
    let report = &observed.report;
    if report.completeness == UsageCompleteness::Partial {
        body.push(card::quiet(text.partial().into(), tokens));
    }
    let bars = report
        .windows
        .as_slice()
        .iter()
        .map(|window| {
            super::source_windows::render(shell, entry, window, view.retained, tokens, cx)
        })
        .chain(report.balances.as_slice().iter().map(|balance| {
            super::source_balances::render(shell, balance, reports.cost_visibility, tokens)
                .into_any_element()
        }))
        .collect::<Vec<_>>();
    if !bars.is_empty() {
        body.push(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .children(bars)
                .into_any_element(),
        );
    }
    let details = report
        .details
        .as_slice()
        .iter()
        .filter(|detail| {
            detail.access == vessel_proto::usage_reports::UsageDetailAccess::Read
                || reports.cost_visibility == vessel_proto::usage::CostVisibility::Permitted
        })
        .map(|detail| {
            let (label, value) = vessel_app_view::usage::detail_text(
                detail.id.as_str(),
                detail.label.as_str(),
                detail.value.as_str(),
                shell.words,
            );
            card::detail(label.into(), value.into(), tokens)
        })
        .collect::<Vec<_>>();
    if !details.is_empty() {
        body.push(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .children(details)
                .into_any_element(),
        );
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{IntoElement, Render, TestAppContext, Window};
    use vessel_app_kit::{Scheme, ui::InteractiveElement as _};
    use vessel_app_view::localization::{ENGLISH, Locale, Translator};
    use vessel_proto::usage_reports::{
        AvailableUsageReport, SuccessfulUsageReport, UsageFetchFailure,
    };
    struct Probe {
        width: f32,
        compact: bool,
        scheme: Scheme,
        locale: Locale,
    }
    fn entry() -> SourceReportEntry {
        let report = AvailableUsageReport {
            plan: Some("Pro".parse().unwrap()),
            ..AvailableUsageReport::default()
        };
        SourceReportEntry::new(
            "cursor:test".parse().unwrap(),
            "Cursor".parse().unwrap(),
            Some(vessel_proto::usage_reports::BuiltinUsageSource::Cursor),
            None,
            Some("테스트 계정 · Test account".parse().unwrap()),
            SourceReportState::Failed {
                attempted_at: None,
                reason: UsageFetchFailure::Unreachable,
                retry_at: None,
                retained: Some(SuccessfulUsageReport {
                    at: chrono::DateTime::UNIX_EPOCH,
                    report,
                }),
                logins: vessel_proto::usage_reports::ReportLoginFailures::default(),
            },
        )
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
            let tokens = Tokens::of(self.scheme);
            let words = Translator::get(self.locale).unwrap();
            let entry = entry();
            let view = source_view(&entry.state, words);
            let head = Head {
                mark: div().into_any_element(),
                name: entry.source_label.as_str().to_owned().into(),
                plan: Some("Pro".into()),
                status: status(&entry, Some(UsageSourceAvailability::Enabled)),
                refresh: Some(
                    super::super::toolbar::refresh_button(
                        "usage-source-head-refresh",
                        words.usage().refresh(),
                        true,
                        false,
                        self.compact,
                        &tokens,
                    )
                    .into_any_element(),
                ),
            };
            let footer = Footer {
                account: Some("테스트 계정 · Test account".into()),
                freshness: self.compact.then(|| {
                    source_freshness(
                        &view,
                        chrono::DateTime::from_timestamp(60, 0).unwrap(),
                        words,
                    )
                    .into()
                }),
            };
            div().w(px(self.width)).child(
                card::frame(head, Vec::new(), footer, words, &tokens)
                    .debug_selector(|| "usage-source-card".to_owned()),
            )
        }
    }
    #[test]
    fn a_failed_attempt_reads_error_and_an_unsupported_source_unavailable() {
        assert_eq!(status(&entry(), None), Some(Status::Error));
        let fresh = SourceReportEntry::new(
            "cursor:test".parse().unwrap(),
            "Cursor".parse().unwrap(),
            None,
            None,
            None,
            SourceReportState::NotRequested,
        );
        assert_eq!(
            status(&fresh, Some(UsageSourceAvailability::Unsupported)),
            Some(Status::Unavailable)
        );
        assert_eq!(status(&fresh, Some(UsageSourceAvailability::Enabled)), None);
    }
    #[gpui::test]
    fn the_shared_card_fits_both_locales_themes_and_compact_widths(cx: &mut TestAppContext) {
        for (width, compact) in [(358.0, true), (608.0, false)] {
            for scheme in [Scheme::Light, Scheme::Dark] {
                for locale in [ENGLISH, Locale::from_tag("ko").unwrap()] {
                    let (_, window) = cx.add_window_view(|_, _| Probe {
                        width,
                        compact,
                        scheme,
                        locale,
                    });
                    window.run_until_parked();
                    let card = window.debug_bounds("usage-source-card").unwrap();
                    assert!(f32::from(card.size.width) <= width);
                    assert!(f32::from(card.size.height) > 0.0);
                }
            }
        }
    }
}
