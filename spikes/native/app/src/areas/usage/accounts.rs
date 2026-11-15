//! Independent account cards disclose policy, retained freshness and safe failures.
use super::{
    draw,
    rows::{Density, message, notice},
};
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, breakpoint::TOUCH_TARGET, outline_button, settings, theme::spacing};
use vessel_app_view::{
    agent_name,
    usage::{
        AccountReport, LimitStatus, Phase, account_binding, limit_failure, limit_view, quota_facts,
        timestamp,
    },
};
use vessel_proto::limits::ExternalLookup;

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
        .children(draw::read_query_status(
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
        return body.child(message(text.empty(), tokens)).into_any_element();
    }
    if state.modal.current().is_none() {
        body = body.children(super::toolbar::refresh_all(
            shell,
            "usage-refresh-all",
            tokens,
            matches!(density, Density::Compact),
            cx,
        ));
    }
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
                    shell.usage_set_policy(machine.clone(), next);
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
    let enabled =
        !matches!(state.inventory.phase(), Phase::Loading { .. }) && state.policy_pending.is_none();
    let title = format!(
        "{} · {}",
        agent_name(account.harness()),
        account.label().unwrap_or(account.profile().as_str())
    );
    let body = account_observation(shell, report, density, tokens, cx);
    let binding = account_binding(account);
    let machine = state.inventory.selection().cloned();
    let refresh = outline_button(
        vessel_app_kit::ui::SharedString::from(format!(
            "usage-account-refresh-{}-{}",
            account.harness(),
            account.profile()
        )),
        text.refresh(),
        enabled,
        tokens,
    )
    .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
    .when(enabled, |button| {
        button.on_click(cx.listener(move |shell, _, _, cx| {
            if let Some(machine) = &machine {
                shell.usage_refresh_account(machine.clone(), binding.clone());
            }
            cx.notify();
        }))
    });
    settings::section_with_trailing(
        title,
        settings::card(tokens).child(body),
        Some(refresh.into_any_element()),
        tokens,
    )
}

fn account_observation(
    shell: &Shell,
    report: &AccountReport,
    density: Density,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let account = &report.account;
    let mut body = div()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .flex()
                .items_center()
                .child(crate::agent_mark::mark(
                    Some(account.harness()),
                    vessel_app_kit::agent_mark::Size::Fourteen,
                    vessel_app_kit::agent_mark::Space::Row,
                    tokens,
                ))
                .child(message(text.account_disclosure(), tokens)),
        );
    match &report.result {
        Err(reason) => {
            body = body.child(notice(
                reason.message(shell.words),
                vessel_app_kit::alert::Variant::Error,
                tokens,
            ));
        }
        Ok(limits) => {
            let view = limit_view(limits);
            let status = match view.status {
                LimitStatus::Off => Some(text.lookup_off()),
                LimitStatus::NotRequested => Some(text.not_requested()),
                LimitStatus::Refreshing => Some(text.loading()),
                LimitStatus::Current => None,
                LimitStatus::Failed(reason) => Some(limit_failure(reason, shell.words)),
            };
            if let Some(status) = status {
                body = body.child(message(status, tokens));
            }
            if let Some(snapshot) = view.snapshot {
                if view.status != LimitStatus::Current {
                    body = body.child(message(text.retained(), tokens));
                }
                if snapshot.incomplete {
                    body = body.child(notice(
                        text.partial(),
                        vessel_app_kit::alert::Variant::Warning,
                        tokens,
                    ));
                }
                let facts = quota_facts(limits, snapshot, shell.words);
                body = body.child(draw::facts(facts, density, tokens)).children(
                    snapshot.windows.iter().map(|window| {
                        super::windows::draw(shell, window, Some(account), tokens, cx)
                    }),
                );
                let at = timestamp(snapshot.at, state.offset, shell.words);
                let freshness = text.refreshed_at(&at).unwrap_or(at);
                body = body.child(message(freshness, tokens));
            }
            if let Some(retry_at) = view.retry_at {
                body = body.child(message(
                    timestamp(retry_at, state.offset, shell.words),
                    tokens,
                ));
            }
        }
    }
    body
}
