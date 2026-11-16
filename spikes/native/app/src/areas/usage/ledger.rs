//! Ledger totals and coverage never masquerade as vendor account allowances.
use super::{
    draw,
    rows::{Density, message, notice},
};
use crate::shell::Shell;
use vessel_app_kit::ui::{AnyElement, IntoElement as _, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::{Tokens, settings, theme::spacing};
use vessel_app_view::{
    agent_name,
    usage::{coverage_facts, partial_report, total_facts},
};

pub(super) fn render(shell: &Shell, tokens: &Tokens, density: Density) -> AnyElement {
    let state = &shell.areas.usage;
    let text = shell.words.usage();
    let mut body = div()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .children(draw::read_status(state.ledger.phase(), shell, tokens));
    let Some(report) = state.ledger.phase().observation() else {
        return body.into_any_element();
    };
    if let Some(reason) = partial_report(report, shell.words) {
        body = body.child(notice(
            reason,
            vessel_app_kit::alert::Variant::Warning,
            tokens,
        ));
    }
    body = body.child(settings::section(
        text.totals(),
        draw::facts(
            total_facts(&report.totals, report.cost_visibility, shell.words),
            density,
            tokens,
        ),
        tokens,
    ));
    for group in &report.agents {
        let agent = agent_name(&group.harness);
        let profile = group
            .profile
            .as_ref()
            .map_or(text.default_account(), |name| name.as_str());
        let title = format!("{agent} · {profile}");
        body = body.child(settings::section(
            title,
            draw::facts(
                total_facts(&group.totals, report.cost_visibility, shell.words),
                density,
                tokens,
            ),
            tokens,
        ));
    }
    body.child(settings::section(
        text.coverage(),
        draw::facts(
            coverage_facts(report, state.offset, shell.words),
            density,
            tokens,
        ),
        tokens,
    ))
    .child(message(text.cost_hint(), tokens))
    .into_any_element()
}
