//! A balance in Paseo's bar shape (balance-bar.tsx): label and amount, the
//! shared meter when an allowance gives a share, then the reset line.
use super::card;
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens,
    ui::{Div, ParentElement as _},
    usage_meter::{self, Appearance, Share},
};
use vessel_app_view::usage::{balance_amount, balance_percent, reset_label};
use vessel_proto::{
    usage::CostVisibility,
    usage_reports::{UsageBalance, UsageTone},
};

pub(super) fn render(
    shell: &Shell,
    balance: &UsageBalance,
    visibility: CostVisibility,
    tokens: &Tokens,
) -> Div {
    // balance-bar.tsx labelRow: the same 12 px medium label and value as a
    // window row, without the pin (balances are never pinned).
    let mut body = card::bar().child(card::label_row(
        vessel_app_view::usage::balance_label_text(
            balance.id.as_str(),
            balance.label.as_str(),
            shell.words,
        ),
        balance_amount(balance, visibility, shell.words),
        None,
        tokens,
    ));
    if let Some(percent) = balance_percent(balance, visibility) {
        let tone = match balance.tone {
            None | Some(UsageTone::Default) => usage_meter::Tone::Default,
            Some(UsageTone::Ok) => usage_meter::Tone::Ok,
            Some(UsageTone::Warning) => usage_meter::Tone::Warning,
            Some(UsageTone::Danger) => usage_meter::Tone::Danger,
        };
        body = body.child(usage_meter::bar(
            Share::percent(percent.whole()),
            Appearance::Normal(tone),
            tokens,
        ));
    }
    if balance.resets_at.is_some() {
        body = body.child(card::reset_line(
            reset_label(
                balance.resets_at,
                shell.areas.usage.offset,
                chrono::Utc::now(),
                shell.words,
            ),
            false,
            tokens,
        ));
    }
    body
}
