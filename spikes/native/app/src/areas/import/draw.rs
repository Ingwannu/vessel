//! Paseo's import entry and adaptive sheet through the shared kit.

use crate::shell::Shell;
use vessel_app_kit::theme::spacing::{S2, S3, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogContent, DialogSize, Tokens, Type, TypeExt as _,
    button_with_kind,
};
use vessel_app_view::history::{History, Loading};

use super::intent::Action;

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
    if !shell.areas.import.open {
        return None;
    }
    let content = DialogContent {
        title: shell.words.history().title().into(),
        body: body(shell, tokens, window, cx).into_any_element(),
        footer: [Action::Refresh, Action::Cancel, Action::Import]
            .into_iter()
            .map(|action| button(action, shell, tokens, cx))
            .collect(),
        size: DialogSize::Normal,
    };
    let modal = vessel_app_kit::dialog(
        &shell.areas.import.dialog,
        content,
        tokens,
        window,
        crate::areas::draft::compact(window),
        super::intent::listener(shell, Action::Cancel),
        cx,
    );
    Some(
        div()
            .child(modal)
            .children(super::pickers::render(shell, tokens, window, cx))
            .into_any_element(),
    )
}

fn body(shell: &Shell, tokens: &Tokens, window: &Window, cx: &mut Context<'_, Shell>) -> Div {
    let area = &shell.areas.import;
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(px(S3))
        .child(super::pickers::controls(shell, tokens, window, cx))
        .child(scope(shell, tokens, cx))
        .child(
            area.search
                .element(tokens, crate::areas::draft::compact(window), cx),
        )
        .children(status(shell, tokens))
        .child(super::rows::list(shell, tokens, window, cx))
        .children(
            area.model
                .has_more()
                .then(|| button(Action::More, shell, tokens, cx)),
        )
}

pub(super) fn state(action: &Action, model: &History) -> ButtonState {
    let ready = match action {
        Action::Cancel => true,
        Action::Row { .. } | Action::Picker(_) | Action::ShowAll => !model.importing(),
        Action::Refresh | Action::More => {
            !model.importing() && !matches!(model.loading(), Loading::Pending(_))
        }
        Action::Import => {
            !model.importing()
                && !matches!(model.loading(), Loading::Pending(_))
                && model.has_selection()
                && !model.requires_refresh()
        }
    };
    if ready {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}

fn button(
    action: Action,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.history();
    let (id, label, kind) = match action {
        Action::Refresh => ("history-refresh", words.refresh(), ButtonKind::Ghost),
        Action::Cancel => ("history-cancel", words.cancel(), ButtonKind::Secondary),
        Action::Import => (
            "history-submit",
            if shell.areas.import.model.importing() {
                words.importing()
            } else {
                words.import()
            },
            ButtonKind::Primary,
        ),
        Action::More => ("history-more", words.more(), ButtonKind::Ghost),
        Action::ShowAll => ("history-show-all", words.show_all(), ButtonKind::Ghost),
        Action::Row { .. } | Action::Picker(_) => return div().into_any_element(),
    };
    let enabled = state(&action, &shell.areas.import.model);
    let listener = super::intent::listener(shell, action);
    button_with_kind(id, label, kind, enabled, tokens)
        .on_click(cx.listener(move |shell, _, window, cx| listener(shell, window, cx)))
        .into_any_element()
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
            body.child(button(Action::ShowAll, shell, tokens, cx))
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
