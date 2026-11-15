//! Layout comes from the existing settings kit; state interpretation is in view.
use super::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled, Window, div, px,
};
use vessel_app_kit::{
    Tokens, Type, TypeExt as _, breakpoint::TOUCH_TARGET, outline_button, settings, theme::spacing,
};
use vessel_app_view::{
    badge::Variant,
    devices::Target,
    sync::{Home, Part, Problem, Reading},
};

#[cfg(test)]
mod tests;

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.sync();
    let state = &shell.areas.settings.sync;
    let title = match &state.page.target {
        Target::Online(machine) | Target::Offline(machine) => words.on_machine(machine.as_str()),
        Target::Hidden | Target::Loading | Target::NoMachines => words.overview.to_owned(),
    };
    let mut body = div().flex().flex_col();
    let mut overview = settings::card(tokens).child(message(words.lead, tokens));
    match &state.page.target {
        Target::Hidden | Target::Loading => {
            overview = overview.child(loading(words.loading, tokens));
        }
        Target::NoMachines => overview = overview.child(message(words.no_machines, tokens)),
        Target::Offline(_) => overview = overview.child(message(words.offline, tokens)),
        Target::Online(_) => {
            match &state.page.reading {
                Reading::Empty | Reading::Loading(_) => {
                    overview = overview.child(loading(words.loading, tokens));
                }
                Reading::Failed(why) => {
                    overview = overview.child(problem(shell, Part::Overview, *why, tokens));
                }
                Reading::Ready(snapshot) => {
                    overview = overview.child(home_row(shell, snapshot, tokens, compact));
                    overview = match &snapshot.published {
                        Ok(published) => overview.child(settings::row_at(
                            words.version(published.generation),
                            Some(words.version_hint.into()),
                            None,
                            None,
                            false,
                            compact,
                            tokens,
                        )),
                        Err(why) => overview.child(problem(shell, Part::Publication, *why, tokens)),
                    };
                }
            }
            let pending = matches!(state.page.reading, Reading::Loading(_));
            let control = outline_button("sync-refresh", words.refresh, !pending, tokens)
                .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
                .when(!pending, |button| {
                    button.on_click(cx.listener(|shell, _event, _window, cx| {
                        shell.sync_read();
                        cx.notify();
                    }))
                })
                .into_any_element();
            overview = overview.child(settings::row_at(
                words.read_controls,
                Some(words.read_hint.into()),
                None,
                Some(control),
                false,
                compact,
                tokens,
            ));
        }
    }
    body = body.child(settings::section(title, overview, tokens));
    if let Reading::Ready(snapshot) = &state.page.reading {
        body = body
            .child(super::shared::publication(
                shell, snapshot, tokens, compact, window,
            ))
            .child(super::members::render(shell, snapshot, tokens, compact))
            .child(super::shared::connections(shell, snapshot, tokens, compact))
            .child(super::shared::secrets(shell, snapshot, tokens, compact));
    }
    if matches!(state.page.target, Target::Online(_) | Target::Offline(_)) {
        body = body.child(super::commands::render(shell, tokens, compact, window, cx));
    }
    body.children(state.tips.render(tokens)).into_any_element()
}

fn home_row(
    shell: &Shell,
    snapshot: &vessel_app_view::sync::Snapshot,
    tokens: &Tokens,
    compact: bool,
) -> Div {
    let words = shell.t.lang.sync();
    let (title, hint, status) = match &snapshot.home {
        Home::Here { name, .. } => (
            name.clone().into(),
            words.home_hint.into(),
            words.home_badge,
        ),
        Home::Unreported(_) => (
            words.home.into(),
            words.home_unknown_hint.into(),
            words.unknown_badge,
        ),
    };
    home_identity(
        title,
        hint,
        status,
        snapshot.home.variant(),
        compact,
        tokens,
    )
}

// The production composition is also the geometry regression's subject:
// only the text grows. A second flex_1 wrapper here used to reserve half
// the row for a leading-aligned badge, moving it when the hint grew.
fn home_identity(
    title: gpui::SharedString,
    hint: gpui::SharedString,
    status: &'static str,
    variant: Variant,
    compact: bool,
    tokens: &Tokens,
) -> Div {
    let selector = title.clone();
    let status = badge(status, variant, tokens)
        .flex_shrink_0()
        .debug_selector(move || format!("sync-home-state-{selector}"));
    settings::row_at(
        title,
        Some(hint),
        None,
        Some(status.into_any_element()),
        false,
        compact,
        tokens,
    )
}

pub(super) fn message(text: impl Into<gpui::SharedString>, tokens: &Tokens) -> Div {
    div()
        .p(px(spacing::S4))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}
pub(super) fn badge(
    label: impl Into<gpui::SharedString>,
    variant: Variant,
    tokens: &Tokens,
) -> Div {
    vessel_app_kit::badge::badge(
        label,
        crate::badge::variant(variant),
        vessel_app_kit::badge::Size::Small,
        None,
        tokens,
    )
}
fn loading(label: &'static str, tokens: &Tokens) -> Div {
    div()
        .p(px(spacing::S4))
        .flex()
        .child(vessel_app_kit::badge::badge(
            label,
            crate::badge::variant(vessel_app_view::sync::LOADING),
            vessel_app_kit::badge::Size::Small,
            Some(vessel_app_kit::slider::spinner(
                "sync-loading",
                spacing::S3,
                tokens.neutral.foreground_muted,
            )),
            tokens,
        ))
}
pub(super) fn problem(shell: &Shell, part: Part, why: Problem, tokens: &Tokens) -> Div {
    div().p(px(spacing::S4)).child(vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: None,
            description: Some(
                div()
                    .child(shell.t.lang.sync().problem(part, why))
                    .into_any_element(),
            ),
            action: None,
        },
        crate::badge::alert_variant(why.notice()),
        vessel_app_kit::alert::Size::Small,
        tokens,
    ))
}
/// Identifiers remain their own wrapping fixed-width element. Flex rather
/// than a fixed width lets a long path share desktop room and stack on touch.
pub(super) fn mono(text: impl Into<gpui::SharedString>, compact: bool) -> Div {
    div()
        .min_w_0()
        .max_w_full()
        .when(compact, Styled::w_full)
        .when(!compact, Styled::flex_1)
        .type_(Type::Mono)
        .whitespace_normal()
        .child(text.into())
}
