//! The same kit meter displays balance ratios; visibility also fences the derived bar.
use super::{
    draw,
    rows::{Density, message},
};
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens, settings,
    theme::spacing,
    ui::{Div, ParentElement as _, Styled as _, div, px},
    usage_meter::{self, Appearance, Share},
};
use vessel_app_view::usage::{balance_facts, balance_percent, reset_label};
use vessel_proto::{
    usage::CostVisibility,
    usage_reports::{UsageBalance, UsageTone},
};

pub(super) fn render(
    shell: &Shell,
    balance: &UsageBalance,
    visibility: CostVisibility,
    density: Density,
    tokens: &Tokens,
) -> Div {
    let mut body = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(draw::facts(
            balance_facts(balance, visibility, shell.words),
            density,
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
        body = body.child(message(
            reset_label(
                balance.resets_at,
                shell.areas.usage.offset,
                chrono::Utc::now(),
                shell.words,
            ),
            tokens,
        ));
    }
    settings::section(balance.label.as_str().to_owned(), body, tokens)
}
