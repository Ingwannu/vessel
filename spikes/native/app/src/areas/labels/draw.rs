//! Draw every manager phase from view state using existing kit furniture.

use vessel_app_kit::alert::{Content, Size, Variant, alert};
use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::theme::spacing::{S2, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_view::labels::{Failure, HostState, ManagerState};

use super::actions::Action;
use crate::shell::Shell;

pub(crate) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.labels;
    if !state.open {
        return None;
    }
    let body = body(shell, tokens, cx);
    Some(dialog(
        &state.dialog,
        DialogContent {
            title: shell.words.labels().manage().into(),
            body: body.into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        vessel_app_kit::breakpoint::phone(window),
        Shell::labels_close,
        cx,
    ))
}

fn body(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let state = &shell.areas.labels;
    let words = shell.words.labels();
    let mut body = div().flex().flex_col().gap(px(S3)).type_(Type::Body);
    let host = state
        .manager
        .selected()
        .and_then(|key| state.hosts.iter().find(|host| &host.key == key));
    body = body.child(hosts(shell, tokens, cx));
    let Some(host) = host else {
        return body.child(if state.unresolved.is_empty() {
            words.no_hosts()
        } else {
            words.connect()
        });
    };
    if let Some(message) = availability(host.state, shell) {
        body = body.child(notice(message, tokens));
    }
    if let Some(failure) = state.error.or_else(|| state.manager.error()) {
        body = body.child(notice(failure_text(failure, shell), tokens));
    }
    if let Some(draft) = state.manager.state().draft() {
        body = body
            .child(state.name.element(tokens, false, cx))
            .child(super::colors::choices(shell, draft.color(), tokens, cx))
            .child(
                button_with_kind(
                    "labels-back",
                    words.back(),
                    ButtonKind::Ghost,
                    enabled(!state.manager.state().busy()),
                    tokens,
                )
                .on_click(cx.listener(|shell, _, _window, cx| {
                    if let Err(failure) = shell.areas.labels.manager.back() {
                        shell.areas.labels.error = Some(failure);
                    }
                    shell.settle_labels(cx);
                    cx.notify();
                })),
            );
    }
    match state.manager.state() {
        ManagerState::Listing => body.child(list(shell, host, tokens, cx)),
        ManagerState::Editing(draft) => {
            let current = host.catalog.get(&draft.original().name);
            let online = host.state == HostState::Online;
            body.child(
                button_with_kind(
                    "labels-save",
                    words.save(),
                    ButtonKind::Primary,
                    enabled(
                        online && current.is_some_and(|label| draft.replacement(label).is_some()),
                    ),
                    tokens,
                )
                .on_click(
                    cx.listener(|shell, _, _window, cx| shell.label_action(Action::Save, cx)),
                ),
            )
            .child(
                button_with_kind(
                    "labels-delete",
                    words.delete(),
                    ButtonKind::Secondary,
                    enabled(online && current.is_some()),
                    tokens,
                )
                .on_click(
                    cx.listener(|shell, _, _window, cx| shell.label_action(Action::Inspect, cx)),
                ),
            )
        }
        ManagerState::Saving { .. } | ManagerState::Deleting { .. } => body.child(words.pending()),
        ManagerState::Inspecting { .. } => body.child(words.inspecting()),
        ManagerState::Confirming { count, .. } => body
            .child(words.confirm())
            .child(
                words
                    .affected(vessel_app_view::localization::Number::count(count.get()))
                    .unwrap_or_else(|_| words.failed().to_owned()),
            )
            .child(
                button_with_kind(
                    "labels-delete-cancel",
                    words.cancel(),
                    ButtonKind::Secondary,
                    ButtonState::Enabled,
                    tokens,
                )
                .on_click(cx.listener(|shell, _, _window, cx| {
                    let _cancelled = shell.areas.labels.manager.cancel_delete();
                    shell.settle_labels(cx);
                    cx.notify();
                })),
            )
            .child(
                button_with_kind(
                    "labels-delete-confirm",
                    words.delete(),
                    ButtonKind::Primary,
                    enabled(host.state == HostState::Online),
                    tokens,
                )
                .on_click(
                    cx.listener(|shell, _, _window, cx| shell.label_action(Action::Delete, cx)),
                ),
            ),
    }
}

const fn enabled(value: bool) -> ButtonState {
    if value {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}

fn list(
    shell: &Shell,
    host: &vessel_app_view::labels::Host,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let state = &shell.areas.labels;
    let words = shell.words.labels();
    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(S3))
        .child(state.search.element(tokens, false, cx));
    let query = state.manager.query().trim().to_lowercase();
    let labels: Vec<_> = host
        .catalog
        .labels()
        .filter(|label| label.name.as_str().to_lowercase().contains(&query))
        .collect();
    if labels.is_empty() && host.state == HostState::Online {
        body = body.child(if query.is_empty() {
            words.empty()
        } else {
            words.no_matches()
        });
    }
    for label in labels {
        let label = label.clone();
        body = body.child(
            button_with_kind(
                label.name.as_str().to_owned(),
                label.name.as_str().to_owned(),
                ButtonKind::Ghost,
                ButtonState::Enabled,
                tokens,
            )
            .child(super::colors::mark(label.color, tokens))
            .on_click(cx.listener(move |shell, _, window, cx| {
                let state = &mut shell.areas.labels;
                match state.manager.edit(label.clone()) {
                    Ok(()) => {
                        state.name.set_text(label.name.as_str(), window, cx);
                        state.name.focus(window, cx);
                    }
                    Err(failure) => state.error = Some(failure),
                }
                shell.settle_labels(cx);
                cx.notify();
            })),
        );
    }
    body
}

use vessel_app_kit::ui::StatefulInteractiveElement as _;

fn hosts(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let state = &shell.areas.labels;
    let mut row = div().flex().flex_wrap().gap(px(S2));
    for host in &state.hosts {
        let key = host.key;
        row = row.child(
            button_with_kind(
                key.to_string(),
                host.route.as_str().to_owned(),
                if state.manager.selected() == Some(&key) {
                    ButtonKind::Primary
                } else {
                    ButtonKind::Secondary
                },
                if state.manager.state().busy() {
                    ButtonState::Disabled
                } else {
                    ButtonState::Enabled
                },
                tokens,
            )
            .on_click(cx.listener(move |shell, _, _window, cx| {
                if let Err(failure) = shell.areas.labels.manager.select(key) {
                    shell.areas.labels.error = Some(failure);
                }
                cx.notify();
            })),
        );
    }
    for route in &state.unresolved {
        row = row.child(button_with_kind(
            route.as_str().to_owned(),
            route.as_str().to_owned(),
            ButtonKind::Secondary,
            ButtonState::Disabled,
            tokens,
        ));
    }
    row
}

fn availability(state: HostState, shell: &Shell) -> Option<&'static str> {
    let words = shell.words.labels();
    match state {
        HostState::Online => None,
        HostState::Offline => Some(words.offline()),
        HostState::Loading => Some(words.loading()),
        HostState::Unsupported => Some(words.unsupported()),
        HostState::Failed(failure) => Some(failure_text(failure, shell)),
    }
}

fn failure_text(failure: Failure, shell: &Shell) -> &'static str {
    let words = shell.words.labels();
    match failure {
        Failure::Offline => words.offline(),
        Failure::Unsupported => words.unsupported(),
        Failure::Pending => words.pending(),
        Failure::NameTaken => words.taken(),
        Failure::NotFound => words.missing(),
        Failure::NoChange => words.invalid(),
        Failure::Load => words.load_failed(),
        Failure::Update | Failure::TicketExhausted => words.failed(),
    }
}

fn notice(message: &'static str, tokens: &Tokens) -> Div {
    alert(
        Content {
            title: Some(message.into()),
            description: None,
            action: None,
        },
        Variant::Info,
        Size::Small,
        tokens,
    )
}
