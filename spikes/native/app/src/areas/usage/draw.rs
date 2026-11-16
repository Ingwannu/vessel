//! The existing Settings page and kit own spacing, controls and scrolling.
use super::{
    accounts, ledger,
    rows::{Density, Fact, Position, message, notice},
};
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{
    Tokens, TypeExt as _,
    breakpoint::TOUCH_TARGET,
    outline_button,
    segmented::{SegmentSize, segment, segmented},
    settings,
    theme::spacing,
};
use vessel_app_view::{
    devices::Target,
    usage::{DisplayAs, Phase, Range},
};

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let density = if compact {
        Density::Compact
    } else {
        Density::Wide
    };
    let body = div().w_full().flex().flex_col().gap(px(spacing::S4));
    match &state.target {
        Target::Hidden | Target::Loading => body
            .child(message(text.loading(), tokens))
            .into_any_element(),
        Target::NoMachines => body
            .child(message(shell.t.lang.settings().no_machines, tokens))
            .into_any_element(),
        Target::Offline(_) => body
            .child(message(text.offline(), tokens))
            .into_any_element(),
        // Paseo's section first (host-usage-section.tsx: title, controls,
        // cards); Vessel's reported-usage ledger is an extra and follows it.
        Target::Online(_) => body
            .child(plan_usage(shell, tokens, density, cx))
            .child(message(text.lead(), tokens))
            .child(controls(shell, tokens, compact, cx))
            .child(ledger::render(shell, tokens, density))
            .into_any_element(),
    }
}

/// Paseo's `UsageSection`: the title with `UsageControls` trailing (Refresh
/// all, then the options cog), the saved-account caveat once, then the cards.
fn plan_usage(
    shell: &Shell,
    tokens: &Tokens,
    density: Density,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let text = shell.words.usage();
    let compact = matches!(density, Density::Compact);
    let trailing = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .children(super::toolbar::refresh_all(
            shell,
            "usage-refresh-all",
            tokens,
            compact,
            cx,
        ))
        .children(super::options::trigger(
            shell,
            "usage-options-settings",
            tokens,
            compact,
            cx,
        ));
    let content = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .child(caveat(shell, tokens))
        .child(accounts::render(shell, tokens, density, cx));
    settings::section_with_trailing(
        text.plan_usage(),
        content,
        Some(trailing.into_any_element()),
        tokens,
    )
}

fn controls(shell: &Shell, tokens: &Tokens, compact: bool, cx: &mut Context<'_, Shell>) -> Div {
    let state = &shell.areas.usage;
    let text = shell.words.usage();
    let mut periods = segmented(SegmentSize::Small).flex_wrap();
    for (index, range) in Range::ALL.into_iter().enumerate() {
        periods = periods.child(
            segment(
                ("usage-period", index),
                range.label(shell.words),
                state.range == range,
                true,
                SegmentSize::Small,
                tokens,
            )
            .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
            .on_click(cx.listener(move |shell, _, _, cx| {
                shell.usage_range(range);
                cx.notify();
            })),
        );
    }
    let busy = matches!(state.ledger.phase(), Phase::Loading { .. });
    let refresh = outline_button("usage-ledger-refresh", text.refresh(), !busy, tokens)
        .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
        .when(!busy, |button| {
            button.on_click(cx.listener(|shell, _, _, cx| {
                shell.usage_refresh();
                cx.notify();
            }))
        });
    let controls = div()
        .min_w_0()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S2))
        .child(periods)
        .child(refresh)
        .child(
            outline_button("usage-open-modal", text.open_modal(), true, tokens)
                .on_click(cx.listener(|shell, _, window, cx| shell.open_usage(window, cx))),
        );
    settings::card(tokens).child(settings::row_at(
        text.period(),
        None,
        None,
        Some(controls.into_any_element()),
        true,
        compact,
        tokens,
    ))
}

/// The saved-account caveat, once under the section title (or at the top of
/// the modal), never inside each card: true for every card, Vessel-specific.
pub(super) fn caveat(shell: &Shell, tokens: &Tokens) -> Div {
    div()
        .ml(px(spacing::S1))
        .type_(vessel_app_kit::Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .child(shell.words.usage().account_disclosure())
}

pub(super) fn read_host_status<T>(
    phase: &Phase<T>,
    shell: &Shell,
    tokens: &Tokens,
) -> Option<AnyElement> {
    if phase.observation().is_some() {
        None
    } else {
        read_query_status(phase, shell, tokens)
    }
}

pub(super) fn read_query_status<T>(
    phase: &Phase<T>,
    shell: &Shell,
    tokens: &Tokens,
) -> Option<AnyElement> {
    if let Phase::Failed { reason, .. } = phase {
        Some(notice(
            reason.message(shell.words),
            vessel_app_kit::alert::Variant::Error,
            tokens,
        ))
    } else {
        read_status(phase, shell, tokens)
    }
}

pub(super) fn read_status<T>(
    phase: &Phase<T>,
    shell: &Shell,
    tokens: &Tokens,
) -> Option<AnyElement> {
    let text = shell.words.usage();
    match phase {
        Phase::Idle => Some(message(text.loading(), tokens).into_any_element()),
        Phase::Loading { retained } => Some(
            message(
                if retained.is_some() {
                    text.retained()
                } else {
                    text.loading()
                },
                tokens,
            )
            .into_any_element(),
        ),
        Phase::Ready(_) => None,
        Phase::Failed { reason, retained } => Some(
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S2))
                .child(notice(
                    reason.message(shell.words),
                    vessel_app_kit::alert::Variant::Error,
                    tokens,
                ))
                .children(retained.as_ref().map(|_| message(text.retained(), tokens)))
                .into_any_element(),
        ),
    }
}

pub(super) fn facts(
    facts: impl IntoIterator<Item = vessel_app_view::usage::ReportFact>,
    density: Density,
    tokens: &Tokens,
) -> Div {
    let mut card = settings::card(tokens);
    for (index, fact) in facts.into_iter().enumerate() {
        card = card.child(
            Fact {
                label: fact.label.into(),
                value: fact.value.into(),
                hint: fact.detail.map(Into::into),
            }
            .draw(
                if index == 0 {
                    Position::First
                } else {
                    Position::Following
                },
                density,
                tokens,
            ),
        );
    }
    card
}

pub(super) fn display_controls(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    receipt: super::options::Opening,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let text = shell.words.usage();
    let mut choices = segmented(SegmentSize::Small).flex_wrap();
    for (index, (value, label)) in [
        (DisplayAs::Used, text.used()),
        (DisplayAs::Remaining, text.remaining()),
    ]
    .into_iter()
    .enumerate()
    {
        let display_receipt = receipt.clone();
        choices = choices.child(
            segment(
                ("usage-display", index),
                label,
                shell.prefs.settings.usage.display == value,
                true,
                SegmentSize::Small,
                tokens,
            )
            .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
            .on_click(cx.listener(move |shell, _, window, cx| {
                if !super::options::admitted(shell, &display_receipt) {
                    return;
                }
                shell.prefer(
                    &vessel_app_view::prefs::PreferenceChange::UsageDisplay(value),
                    window,
                    cx,
                );
            })),
        );
    }
    let shown = super::sidebar::visible(shell);
    let visibility = settings::switch("usage-show-sidebar", shown, true, compact, 0, tokens)
        .on_click(cx.listener(move |shell, _, window, cx| {
            if super::options::admitted(shell, &receipt) {
                shell.usage_show_sidebar(!shown, window, cx);
            }
        }));
    div()
        .child(settings::row_at(
            text.show_sidebar(),
            None,
            None,
            Some(visibility.into_any_element()),
            false,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            text.display(),
            None,
            None,
            Some(choices.into_any_element()),
            false,
            compact,
            tokens,
        ))
}
