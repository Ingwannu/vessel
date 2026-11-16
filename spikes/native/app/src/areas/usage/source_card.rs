//! Canonical report cards reuse the shared card/row, badge, dot and tooltip controls.
use super::rows::{Density, message, notice};
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens, Type, TypeExt as _, settings,
    theme::spacing,
    ui::{Context, Div, IntoElement as _, ParentElement as _, Styled as _, div, px},
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
    let view = source_view(&entry.state, shell.words);
    let mut body = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2));
    let login_errors = source_login_errors(&entry.state, shell.words);
    if login_errors.is_empty() {
        if let Some(status) = &view.status {
            body = body.child(message(status.clone(), tokens));
        }
    } else {
        for error in login_errors {
            body = body.child(notice(
                error,
                vessel_app_kit::alert::Variant::Warning,
                tokens,
            ));
        }
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
    let receipt = super::read_states::context(shell);
    let id = entry.id().clone();
    let busy = matches!(state.inventory.phase(), Phase::Loading { .. });
    let refresh = super::toolbar::refresh_button(
        vessel_app_kit::ui::SharedString::from(format!("usage-report-refresh-{id}")),
        text.refresh(),
        enabled,
        busy,
        matches!(density, Density::Compact),
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
    let freshness = source_freshness(&view, chrono::Utc::now(), shell.words);
    let refresh = state.source_tips.on(refresh, entry, &freshness);
    let head = header(entry, &view, shell.words, tokens);
    let footer = footer(entry, freshness, shell.words, tokens);
    settings::card(tokens)
        .child(settings::row_with_content(
            head.into_any_element(),
            Some(refresh.into_any_element()),
            true,
            matches!(density, Density::Compact),
            tokens,
        ))
        .child(body)
        .child(settings::row_with_content(
            footer.into_any_element(),
            None,
            false,
            matches!(density, Density::Compact),
            tokens,
        ))
}

fn footer(
    entry: &SourceReportEntry,
    freshness: String,
    words: &'static vessel_app_view::localization::Translator,
    tokens: &Tokens,
) -> Div {
    let text = words.usage();
    let mut footer = div().min_w_0().flex().flex_col().gap(px(spacing::S1));
    if let Some(account) = &entry.account_label {
        footer = footer.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(account.as_str().to_owned()),
        );
    }
    footer = footer.child(
        div()
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .child(freshness),
    );
    if matches!(entry.state, SourceReportState::Failed { .. }) {
        footer = footer.child(message(text.refresh_failed(), tokens));
    }
    footer
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
    }
    body
}

fn header(
    entry: &SourceReportEntry,
    view: &vessel_app_view::usage::SourceView<'_>,
    words: &'static vessel_app_view::localization::Translator,
    tokens: &Tokens,
) -> Div {
    let mut head = div()
        .min_w_0()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S2))
        .child(vessel_app_kit::agent_mark::agent_mark(
            crate::agent_mark::artwork(vessel_app_view::usage::builtin_source_mark(entry.builtin)),
            vessel_app_kit::agent_mark::Size::Fourteen,
            vessel_app_kit::agent_mark::Space::Row,
            tokens,
        ))
        .child(
            div()
                .min_w_0()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(entry.source_label.as_str().to_owned()),
        );
    if let Some(plan) = view
        .observed
        .and_then(|observed| observed.report.plan.as_ref())
    {
        head = head.child(vessel_app_kit::badge::badge(
            plan.as_str().to_owned(),
            vessel_app_kit::badge::Variant::Muted,
            vessel_app_kit::badge::Size::ExtraSmall,
            None,
            tokens,
        ));
    }
    let signal = match entry.state {
        SourceReportState::Failed { .. } => Some((
            words.usage().status_error(),
            tokens.status().error.foreground,
        )),
        SourceReportState::Unavailable { .. } => Some((
            words.usage().status_unavailable(),
            tokens.status().warning.foreground,
        )),
        SourceReportState::Refreshing { .. } => {
            Some((words.usage().loading(), tokens.status().running_dot))
        }
        SourceReportState::NotRequested | SourceReportState::Available { .. } => None,
    };
    if let Some((label, color)) = signal {
        head = head.child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::S1))
                .child(vessel_app_kit::dot(color))
                .child(div().type_(Type::Meta).text_color(color).child(label)),
        );
    }
    head
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
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
            let tokens = Tokens::of(self.scheme);
            let words = Translator::get(self.locale).unwrap();
            let report = AvailableUsageReport {
                plan: Some("Pro".parse().unwrap()),
                ..AvailableUsageReport::default()
            };
            let entry = SourceReportEntry::new(
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
            );
            let view = source_view(&entry.state, words);
            div().w(px(self.width)).child(
                settings::card(&tokens)
                    .child(settings::row_with_content(
                        header(&entry, &view, words, &tokens)
                            .debug_selector(|| "usage-source-head".to_owned())
                            .into_any_element(),
                        Some(
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
                        true,
                        self.compact,
                        &tokens,
                    ))
                    .child(message(
                        source_freshness(
                            &view,
                            chrono::DateTime::from_timestamp(60, 0).unwrap(),
                            words,
                        ),
                        &tokens,
                    )),
            )
        }
    }
    #[gpui::test]
    fn shared_report_header_and_footer_fit_both_locales_themes_and_compact_widths(
        cx: &mut TestAppContext,
    ) {
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
                    let head = window.debug_bounds("usage-source-head").unwrap();
                    assert!(f32::from(head.size.width) <= width);
                    assert!(f32::from(head.size.height) > 0.0);
                }
            }
        }
    }
}
