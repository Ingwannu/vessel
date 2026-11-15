//! Paseo's import entry and adaptive sheet through the shared kit.

use super::Picker;
use crate::shell::Shell;
use vessel_app_kit::theme::spacing::{S2, S3, S4, S11};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogContent, DialogSize, Tokens, Type, TypeExt as _,
    button_with_kind,
};
use vessel_app_view::history::Loading;

/// The new-conversation affordance mirrors Paseo's import pill.
pub(crate) fn entry(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    button_with_kind(
        "history-open",
        shell.words.history().title(),
        ButtonKind::Secondary,
        ButtonState::Enabled,
        tokens,
    )
    .on_click(cx.listener(|shell, _, window, cx| shell.open_import(window, cx)))
    .into_any_element()
}

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if shell.areas.import.pending_open.is_some() {
        cx.defer_in(window, Shell::history_open_pending);
    }
    if !shell.areas.import.open {
        return None;
    }
    let area = &shell.areas.import;
    let words = shell.words.history();
    let importing = area.model.importing();
    let reading = matches!(area.model.loading(), Loading::Pending(_));
    let can_import =
        !importing && !reading && area.model.has_selection() && !area.model.requires_refresh();
    let body = div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(px(S3))
        .child(pickers(shell, tokens, window, cx))
        .child(scope(shell, tokens, cx))
        .child(
            area.search
                .element(tokens, crate::areas::draft::compact(window), cx),
        )
        .children(status(shell, tokens))
        .child(super::rows::list(shell, tokens, window, cx))
        .children(area.model.has_more().then(|| {
            button_with_kind(
                "history-more",
                words.more(),
                ButtonKind::Ghost,
                enabled(!reading && !importing),
                tokens,
            )
            .on_click(cx.listener(|shell, _, _, cx| {
                shell.history_more();
                cx.notify();
            }))
            .into_any_element()
        }));
    let footer = vec![
        button_with_kind(
            "history-refresh",
            words.refresh(),
            ButtonKind::Ghost,
            enabled(!reading && !importing),
            tokens,
        )
        .on_click(cx.listener(|shell, _, _, cx| {
            if !shell.areas.import.model.importing() {
                shell.history_profiles();
                shell.history_refresh();
            }
            cx.notify();
        }))
        .into_any_element(),
        button_with_kind(
            "history-cancel",
            words.cancel(),
            ButtonKind::Secondary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _, window, cx| shell.close_import(window, cx)))
        .into_any_element(),
        button_with_kind(
            "history-submit",
            if importing {
                words.importing()
            } else {
                words.import()
            },
            ButtonKind::Primary,
            enabled(can_import),
            tokens,
        )
        .on_click(cx.listener(|shell, _, _, cx| {
            shell.history_import();
            cx.notify();
        }))
        .into_any_element(),
    ];
    let modal = vessel_app_kit::dialog(
        &area.dialog,
        DialogContent {
            title: words.title().into(),
            body: body.into_any_element(),
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        Shell::close_import,
        cx,
    );
    Some(
        div()
            .child(modal)
            .children(super::pickers::render(shell, tokens, window, cx))
            .into_any_element(),
    )
}

fn enabled(available: bool) -> ButtonState {
    if available {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}

fn pickers(shell: &Shell, tokens: &Tokens, window: &Window, cx: &mut Context<'_, Shell>) -> Div {
    let area = &shell.areas.import;
    let words = shell.words.history();
    let scope = area.model.scope();
    let machine = scope
        .map_or(words.choose_machine(), |scope| scope.machine.as_str())
        .to_owned();
    let agent = scope.map_or_else(
        || words.choose_agent().to_owned(),
        |scope| vessel_app_view::agent_name(&scope.harness),
    );
    let profile = scope
        .and_then(|scope| scope.profile.as_ref())
        .map_or_else(|| words.default_account().to_owned(), ToString::to_string);
    let touch = crate::areas::draft::compact(window);
    let active = !area.model.importing();
    let fields = [
        (
            Picker::Machine,
            words.machine(),
            machine,
            &area.machine_anchor,
        ),
        (Picker::Agent, words.agent(), agent, &area.agent_anchor),
        (
            Picker::Account,
            words.account(),
            profile,
            &area.account_anchor,
        ),
    ];
    div()
        .flex()
        .flex_col()
        .gap(px(S2))
        .children(fields.into_iter().map(|(which, label, value, anchor)| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(S2))
                .child(
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(label),
                )
                .child(
                    vessel_app_kit::select::trigger(
                        format!("history-{which:?}"),
                        value,
                        anchor,
                        area.choosing == Some(which) && area.picker.is_open(),
                        active,
                        touch,
                        tokens,
                    )
                    .min_h(px(S11))
                    .on_click(cx.listener(move |shell, _, window, cx| {
                        shell.history_picker(which, window, cx)
                    })),
                )
        }))
}

fn scope(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.history();
    let scoped = shell
        .areas
        .import
        .model
        .scope()
        .is_some_and(|scope| scope.cwd.is_some());
    div()
        .flex()
        .flex_col()
        .gap(px(S2))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(if scoped { words.scoped() } else { words.all() }),
        )
        .when(scoped, |body| {
            body.child(
                button_with_kind(
                    "history-show-all",
                    words.show_all(),
                    ButtonKind::Ghost,
                    enabled(!shell.areas.import.model.importing()),
                    tokens,
                )
                .on_click(cx.listener(|shell, _, _, cx| {
                    shell.history_show_all();
                    cx.notify();
                })),
            )
        })
}

fn status(shell: &Shell, tokens: &Tokens) -> Vec<AnyElement> {
    let area = &shell.areas.import;
    let words = shell.words.history();
    let mut messages = Vec::new();
    if let Loading::Failed(problem) = area.model.loading() {
        messages.push(alert(problem.words(shell.words), tokens));
    }
    if let Some(problem) = area.failure {
        messages.push(alert(problem.words(shell.words), tokens));
    }
    if let Err(problem) = &area.profiles {
        messages.push(alert(problem.words(shell.words), tokens));
    }
    let mut reported = Vec::new();
    for receipt in area.model.outcomes() {
        if let Err(problem) = receipt.result
            && !reported.contains(&problem)
        {
            reported.push(problem);
            messages.push(alert(problem.words(shell.words), tokens));
        }
    }
    if matches!(area.model.loading(), Loading::Pending(_)) {
        messages.push(
            div()
                .type_(Type::Meta)
                .child(words.loading())
                .into_any_element(),
        );
    } else if area.model.scope().is_none() {
        messages.push(alert(words.no_machine(), tokens));
    } else if area.visible.is_empty() && !matches!(area.model.loading(), Loading::Failed(_)) {
        messages.push(
            div()
                .type_(Type::Body)
                .p(px(S4))
                .child(if area.query.is_empty() {
                    words.empty()
                } else {
                    words.no_matches()
                })
                .into_any_element(),
        );
    }
    messages
}

fn alert(message: &str, tokens: &Tokens) -> AnyElement {
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: Some(message.to_owned().into()),
            description: None,
            action: None,
        },
        vessel_app_kit::alert::Variant::Error,
        vessel_app_kit::alert::Size::Small,
        tokens,
    )
    .into_any_element()
}
