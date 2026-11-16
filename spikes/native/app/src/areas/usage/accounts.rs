//! Independent account cards disclose policy, retained freshness and safe failures.
use super::{
    card::{self, Footer, Head, Status},
    draw,
    rows::{Density, notice},
};
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, settings, theme::spacing};
use vessel_app_view::{
    agent_name,
    usage::{
        AccountReport, InventoryRefresh, LimitStatus, Phase, RequestFailure, account_binding,
        freshness, limit_failure, limit_view, quota_details, timestamp,
    },
};
use vessel_proto::limits::{ExternalLookup, LimitFailure};

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    density: Density,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = &shell.areas.usage;
    let text = shell.words.usage();
    let mut body = div()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .children(draw::read_host_status(
            state.inventory.phase(),
            shell,
            tokens,
        ))
        .children(super::read_states::retry(shell, tokens, cx));
    if let Some(error) = state.policy_error {
        body = body.child(notice(
            error.message(shell.words),
            vessel_app_kit::alert::Variant::Error,
            tokens,
        ));
    }
    let Some(inventory) = state.inventory.phase().observation() else {
        return body.into_any_element();
    };
    if let Some(error) = inventory.refresh_problem() {
        body = body.child(notice(
            error.message(shell.words),
            vessel_app_kit::alert::Variant::Error,
            tokens,
        ));
    }
    if let Some(reports) = inventory.source_reports() {
        return body
            .child(super::source_reports::render(
                shell, reports, density, tokens, cx,
            ))
            .into_any_element();
    }
    if inventory.policy() == Some(ExternalLookup::Off) {
        return body
            .child(super::read_states::off(shell, tokens, cx))
            .into_any_element();
    }
    if inventory.accounts().is_empty() {
        return body
            .child(super::read_states::muted_card(text.plan_empty(), tokens))
            .into_any_element();
    }
    // Paseo list.tsx: one card per report, gap 12. Refresh all lives in the
    // section's title row (draw::plan_usage) or the modal header.
    body = body.gap(px(spacing::S3));
    for account in inventory.accounts() {
        body = body.child(account_card(shell, account, density, tokens, cx));
    }
    body.into_any_element()
}

pub(super) fn policy_fields(
    shell: &Shell,
    policy: Option<ExternalLookup>,
    tokens: &Tokens,
    density: Density,
    receipt: super::options::Opening,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let compact = matches!(density, Density::Compact);
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let enabled = state.policy_pending.is_none() && state.source_pending.is_none();
    let machine = state.inventory.selection().cloned();
    let policy_receipt = receipt.clone();
    let action = policy.map(|policy| {
        let next = match policy {
            ExternalLookup::Off => ExternalLookup::On,
            ExternalLookup::On => ExternalLookup::Off,
        };
        settings::switch(
            "usage-lookup-policy",
            policy == ExternalLookup::On,
            enabled,
            compact,
            0,
            tokens,
        )
        .when(enabled, |button| {
            button.on_click(cx.listener(move |shell, _, _, cx| {
                if !super::options::admitted(shell, &policy_receipt) {
                    return;
                }
                if let Some(machine) = &machine {
                    shell.usage_set_policy(machine, next);
                }
                cx.notify();
            }))
        })
        .into_any_element()
    });
    div()
        .child(settings::row_at(
            text.lookup(),
            Some(text.lookup_disclosure().into()),
            None,
            action,
            true,
            compact,
            tokens,
        ))
        .child(draw::display_controls(shell, tokens, compact, receipt, cx))
}

/// One saved sign-in in Paseo's card shape (card.rs): its harness mark and
/// name, the plan badge, Error or Unavailable beside the dot, and an icon
/// Refresh whose tooltip carries the freshness. Unsupported harnesses get no
/// Refresh: nothing could be read.
fn account_card(
    shell: &Shell,
    report: &AccountReport,
    density: Density,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let account = &report.account;
    let compact = matches!(density, Density::Compact);
    let view = report.result.as_ref().ok().map(limit_view);
    let latest = view.as_ref().map(|view| view.status);
    let unsupported = matches!(report.result, Err(RequestFailure::Unsupported))
        || latest == Some(LimitStatus::Failed(LimitFailure::Unsupported));
    let status = if unsupported {
        Some(Status::Unavailable)
    } else if report.result.is_err() || matches!(latest, Some(LimitStatus::Failed(_))) {
        Some(Status::Error)
    } else {
        None
    };
    let snapshot = view.as_ref().and_then(|view| view.snapshot);
    let retained = snapshot.is_some() && latest != Some(LimitStatus::Current);
    let freshness = freshness(
        snapshot.map(|snapshot| snapshot.at),
        retained,
        chrono::Utc::now(),
        shell.words,
    );
    let key = format!("{}-{}", account.harness(), account.profile());
    let refresh = (!unsupported).then(|| {
        let binding = account_binding(account);
        let loading = matches!(state.inventory.phase(), Phase::Loading { .. });
        let busy = latest == Some(LimitStatus::Refreshing)
            || (loading && state.refresh_account == InventoryRefresh::Account(binding.clone()));
        let enabled = !loading && state.policy_pending.is_none();
        let machine = state.inventory.selection().cloned();
        let button = super::toolbar::refresh_button(
            SharedString::from(format!("usage-account-refresh-{key}")),
            text.refresh(),
            enabled,
            busy,
            compact,
            tokens,
        )
        .on_click(cx.listener(move |shell, _, _, cx| {
            if let Some(machine) = &machine {
                shell.usage_refresh_account(machine, binding.clone());
            }
            cx.notify();
        }));
        state
            .source_tips
            .on_saved(button, &key, &freshness)
            .into_any_element()
    });
    let head = Head {
        mark: crate::agent_mark::mark(
            Some(account.harness()),
            vessel_app_kit::agent_mark::Size::Fourteen,
            vessel_app_kit::agent_mark::Space::Row,
            tokens,
        )
        .into_any_element(),
        name: agent_name(account.harness()).into(),
        plan: snapshot
            .and_then(|snapshot| snapshot.plan.as_ref())
            .map(|plan| plan.as_str().to_owned().into()),
        status,
        refresh,
    };
    let footer = Footer {
        account: Some(
            account
                .label()
                .unwrap_or(account.profile().as_str())
                .to_owned()
                .into(),
        ),
        freshness: compact.then(|| freshness.into()),
    };
    card::frame(
        head,
        account_body(shell, report, unsupported, tokens, cx),
        footer,
        shell.words,
        tokens,
    )
}

/// Errors, the not-yet-checked line, the windows and the remaining facts.
fn account_body(
    shell: &Shell,
    report: &AccountReport,
    unsupported: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let account = &report.account;
    let mut body = Vec::new();
    let limits = match &report.result {
        Err(_) if unsupported => return body,
        Err(reason) => {
            body.push(card::error(reason.message(shell.words).into(), tokens));
            return body;
        }
        Ok(limits) => limits,
    };
    let view = limit_view(limits);
    match view.status {
        LimitStatus::NotRequested => body.push(card::quiet(text.not_requested().into(), tokens)),
        LimitStatus::Off => body.push(card::quiet(text.lookup_off().into(), tokens)),
        LimitStatus::Failed(reason) if !unsupported => {
            body.push(card::error(
                limit_failure(reason, shell.words).into(),
                tokens,
            ));
        }
        LimitStatus::Failed(_) | LimitStatus::Refreshing | LimitStatus::Current => {}
    }
    if let Some(snapshot) = view.snapshot {
        if snapshot.incomplete {
            body.push(card::quiet(text.partial().into(), tokens));
        }
        if !snapshot.windows.is_empty() {
            body.push(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::S1))
                    .children(snapshot.windows.iter().map(|window| {
                        super::windows::draw(shell, window, Some(account), tokens, cx)
                    }))
                    .into_any_element(),
            );
        }
        let details = quota_details(limits, snapshot, shell.words);
        if !details.is_empty() {
            body.push(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::S1))
                    .children(
                        details
                            .into_iter()
                            .map(|fact| card::detail(fact.label.into(), fact.value.into(), tokens)),
                    )
                    .into_any_element(),
            );
        }
    }
    if let Some(retry_at) = view.retry_at {
        body.push(card::quiet(
            timestamp(retry_at, state.offset, shell.words).into(),
            tokens,
        ));
    }
    body
}
