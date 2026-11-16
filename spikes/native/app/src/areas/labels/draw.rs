//! Draw every manager phase from view state using existing kit furniture.

use vessel_app_kit::alert::{Content, Size, Variant, alert};
use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::theme::spacing::{S2, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_view::labels::{Failure, HostState, ManagerState};

use crate::shell::Shell;

pub(crate) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.labels;
    let opening = state.opening?;
    let dismiss = super::surface::Dismiss::capture(opening, state.manager.state());
    let body = body(shell, opening, tokens, cx);
    Some(dialog(
        &state.dialog,
        DialogContent {
            title: title(shell).into(),
            body: body.into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        vessel_app_kit::breakpoint::phone(window),
        move |shell, window, cx| shell.labels_close(dismiss, window, cx),
        cx,
    ))
}

fn title(shell: &Shell) -> String {
    let words = shell.words.labels();
    match shell.areas.labels.manager.state() {
        ManagerState::Listing => words.manage().to_owned(),
        ManagerState::Editing(_)
        | ManagerState::Saving { .. }
        | ManagerState::Inspecting { .. } => words.edit().to_owned(),
        ManagerState::Confirming { draft, .. } | ManagerState::Deleting { draft, .. } => words
            .delete_title(draft.original().name.as_str())
            .unwrap_or_else(|_| words.confirm().to_owned()),
    }
}

fn body(
    shell: &Shell,
    opening: super::surface::Opening,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let state = &shell.areas.labels;
    let words = shell.words.labels();
    let mut body = div().flex().flex_col().gap(px(S3)).type_(Type::Body);
    let host = state.selected_host();
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
    if matches!(state.manager.state(), ManagerState::Confirming { .. })
        && state.confirmation.as_ref() != host.connection.as_ref()
    {
        body = body.child(notice(words.changed(), tokens));
    }
    if let Some(failure) = state.error.or_else(|| state.manager.error()) {
        body = body.child(notice(failure_text(failure, shell), tokens));
    }
    match state.manager.state() {
        ManagerState::Listing => body.child(list(shell, host, tokens, cx)),
        ManagerState::Editing(draft)
        | ManagerState::Saving { draft, .. }
        | ManagerState::Inspecting { draft, .. }
        | ManagerState::Confirming { draft, .. }
        | ManagerState::Deleting { draft, .. } => {
            let connection = match state.manager.state() {
                ManagerState::Confirming { .. } | ManagerState::Deleting { .. } => {
                    state.confirmation.clone()
                }
                ManagerState::Listing
                | ManagerState::Editing(_)
                | ManagerState::Saving { .. }
                | ManagerState::Inspecting { .. } => host.connection.clone(),
            };
            let target = super::actions::Target::new(
                opening,
                host.key,
                connection,
                draft.original().name.clone(),
            );
            body.child(super::form::editor(shell, host, draft, &target, tokens, cx))
        }
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
        .child(state.search.element(
            state.search.id().element_id("label-manager-search"),
            tokens,
            cx,
        ));
    let labels = state.manager.filtered(&host.catalog);
    if labels.is_empty() && host.state == HostState::Online {
        body = body.child(if state.manager.has_query() {
            words.no_matches()
        } else {
            words.empty()
        });
    }
    for label in labels {
        let label = label.clone();
        let route = host.route.clone();
        let connection = host.connection.clone();
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
                if !state
                    .selected_host()
                    .is_some_and(|host| host.route == route && host.connection == connection)
                {
                    return;
                }
                match state.manager.edit(label.clone()) {
                    Ok(()) => {
                        state.name =
                            super::fields::manager(shell.words.labels().name(), window, cx);
                        state.applied_editability = None;
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
        let route = host.route.clone();
        let connection = host.connection.clone();
        row = row.child(
            button_with_kind(
                host.route.as_str().to_owned(),
                host.route.as_str().to_owned(),
                if state.route.as_ref() == Some(&route) {
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
            .when(!state.manager.state().busy(), |button| {
                button.on_click(cx.listener(move |shell, _, _window, cx| {
                    if !shell
                        .areas
                        .labels
                        .hosts
                        .iter()
                        .any(|host| host.route == route && host.connection == connection)
                    {
                        return;
                    }
                    if let Err(failure) = shell.areas.labels.select_route(&route) {
                        shell.areas.labels.error = Some(failure);
                    }
                    cx.notify();
                }))
            }),
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

pub(super) fn failure_text(failure: Failure, shell: &Shell) -> &'static str {
    let words = shell.words.labels();
    match failure {
        Failure::Offline => words.offline(),
        Failure::Changed => words.changed(),
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
