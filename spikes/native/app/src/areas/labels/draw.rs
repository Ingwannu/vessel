//! Draw every manager phase after Paseo's `workspace-labels/manager-modal.tsx`
//! (120-357) on the kit dialog.
//!
//! [Decision Log]
//! - Purpose: the manager's body is Paseo's (the coordinator's RETURN on the
//!   W10b body, 2026-11-16): the host filter only with more than one host,
//!   one left rail of rows (the colour dot, the name, a pencil that fades in
//!   on hover), Paseo's empty, offline and update lines, and an edit view of
//!   the name and the swatch row with the way back in the header and Delete
//!   and Save in the footer.
//! - Before: host-named buttons, centred Ghost rows with the dot after the
//!   name, info alerts for every status, a body Back and stacked Save/Delete.
//! - Choice: the kit dialog's own header search (listing) and header back
//!   (editing), its footer slot for the actions, the kit combobox for the host
//!   list; the row, its pencil and the status lines are drawn here because
//!   only this manager has them.
//! - Kept: every admission check (opening, host, connection, ticket) of the
//!   W10b actions, and the delete confirmation as its own phase. The
//!   confirmation shows Paseo's `confirmDialog` content (the question, the
//!   count) with Cancel and Delete in the footer, in place of the edit view,
//!   as the kit has no dialog over a dialog.
//! - Differences: the kit button has no loading state, so a pending save or
//!   check shows a muted line under the swatches rather than a spinner in
//!   Save; the pencil's and the swatches' `hitSlop` is not reproduced (Paseo's
//!   web build has none).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{DialogContent, DialogInput, DialogSize, HeaderBack, dialog};
use vessel_app_kit::theme::spacing::{S1, S2, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, SharedString, Stateful, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};
use vessel_app_view::labels::{Failure, Host, HostState, Label, ManagerState};

use super::actions::Target;
use crate::shell::Shell;

/// `rowEdit`: the pencil glyph (`EDIT_ICON_SIZE`).
const PENCIL: f32 = 14.0;
/// `row.minHeight`.
const ROW_MIN: f32 = 32.0;
/// `rowEditPressed.opacity`.
const PRESSED: f32 = 0.6;

pub(crate) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.labels;
    let opening = state.opening?;
    let dismiss = super::surface::Dismiss::capture(opening, state.manager.state());
    let narrow = vessel_app_kit::breakpoint::phone(window);
    let target = target(shell, opening);
    let body = body(shell, target.as_ref(), narrow, tokens, cx);
    let footer = target
        .as_ref()
        .zip(state.selected_host())
        .map_or_else(Vec::new, |(target, host)| {
            super::form::footer(shell, host, target, tokens, cx)
        });
    let content = DialogContent {
        title: title(shell).into(),
        body: body.into_any_element(),
        footer,
        size: DialogSize::Normal,
    };
    // Paseo's manager gives its query to the sheet header while listing and
    // its way back to the header while editing (manager-modal.tsx 120-141).
    let content: DialogInput = match (state.manager.state(), target) {
        (ManagerState::Listing, _) => {
            content.with_header_search(state.search.header_search(tokens, cx))
        }
        (
            ManagerState::Editing(_)
            | ManagerState::Saving { .. }
            | ManagerState::Inspecting { .. },
            Some(target),
        ) => content.with_header_back(back(target, cx)),
        (
            ManagerState::Editing(_)
            | ManagerState::Saving { .. }
            | ManagerState::Inspecting { .. }
            | ManagerState::Confirming { .. }
            | ManagerState::Deleting { .. },
            _,
        ) => content.into(),
    };
    let modal = dialog(
        &state.dialog,
        content,
        tokens,
        window,
        narrow,
        move |shell, window, cx| shell.labels_close(dismiss, window, cx),
        cx,
    );
    Some(
        div()
            .child(modal)
            .children(super::host_filter::list(shell, tokens, window, cx))
            .into_any_element(),
    )
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

/// The draft's admission: the opening, the host and the connection it was
/// opened on (a confirmation keeps the one it inspected on).
fn target(shell: &Shell, opening: super::surface::Opening) -> Option<Target> {
    let state = &shell.areas.labels;
    let host = state.selected_host()?;
    let (draft, connection) = match state.manager.state() {
        ManagerState::Listing => return None,
        ManagerState::Editing(draft)
        | ManagerState::Saving { draft, .. }
        | ManagerState::Inspecting { draft, .. } => (draft, host.connection.clone()),
        ManagerState::Confirming { draft, .. } | ManagerState::Deleting { draft, .. } => {
            (draft, state.confirmation.clone())
        }
    };
    Some(Target::new(
        opening,
        host.key,
        connection,
        draft.original().name.clone(),
    ))
}

/// Paseo's header back (`cancelEdit`): leaves the edit view for the list;
/// nothing while a save or check is under way.
fn back(target: Target, cx: &mut Context<'_, Shell>) -> HeaderBack {
    HeaderBack::new(cx.listener(move |shell, _event, _window, cx| {
        let state = &mut shell.areas.labels;
        if state.manager.state().busy()
            || !target.matches(
                state.opening,
                &state.manager,
                state
                    .selected_host()
                    .and_then(|host| host.connection.as_ref()),
            )
        {
            return;
        }
        if let Err(failure) = state.manager.back() {
            state.error = Some(failure);
        }
        shell.settle_labels(cx);
        cx.notify();
    }))
}

fn body(
    shell: &Shell,
    target: Option<&Target>,
    narrow: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let state = &shell.areas.labels;
    let words = shell.words.labels();
    // Paseo's sheet content: its children 16 apart.
    let body = div()
        .debug_selector(|| "labels-body".to_owned())
        .flex()
        .flex_col()
        .gap(px(S4))
        .type_(Type::Body);
    let Some(host) = state.selected_host() else {
        return body.child(muted(
            if state.unresolved.is_empty() {
                words.no_hosts()
            } else {
                words.connect()
            },
            tokens,
        ));
    };
    match (state.manager.state(), target) {
        (ManagerState::Listing, _) | (_, None) => listing(shell, host, narrow, tokens, cx, body),
        (
            ManagerState::Editing(draft)
            | ManagerState::Saving { draft, .. }
            | ManagerState::Inspecting { draft, .. },
            Some(target),
        ) => body
            .child(super::form::editor(shell, draft, target, tokens, cx))
            .children(pending(shell, tokens))
            .children(errors(shell, host, tokens)),
        (ManagerState::Confirming { count, .. }, Some(_)) => body
            .child(super::form::confirmation(shell, *count))
            .children(changed(shell, host, tokens))
            .children(errors(shell, host, tokens)),
        (ManagerState::Deleting { .. }, Some(_)) => body
            .child(muted(words.pending(), tokens))
            .children(errors(shell, host, tokens)),
    }
}

/// Paseo's list view: the host filter with more than one host, the rows,
/// then the empty, offline, update and error lines in its order.
fn listing(
    shell: &Shell,
    host: &Host,
    narrow: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
    body: Div,
) -> Div {
    let state = &shell.areas.labels;
    let words = shell.words.labels();
    let labels = state.manager.filtered(&host.catalog);
    // Paseo: `state.host?.status !== "online" || state.pending`.
    let enabled = host.state == HostState::Online && !state.manager.state().busy();
    let mut list = div()
        .debug_selector(|| "labels-list".to_owned())
        .flex()
        .flex_col()
        .gap(px(S1));
    // Paseo shows its one empty line for no labels and for no match alike.
    if labels.is_empty() {
        list = list.child(muted(words.empty(), tokens));
    }
    for label in labels {
        list = list.child(row(shell, host, label, enabled, narrow, tokens, cx));
    }
    body.when(super::host_filter::shown(state), |body| {
        body.child(super::host_filter::trigger(shell, tokens, cx))
    })
    .child(list)
    .children(match host.state {
        HostState::Online | HostState::Failed(_) => None,
        HostState::Offline => Some(muted(words.offline(), tokens)),
        HostState::Loading => Some(muted(words.loading(), tokens)),
        HostState::Unsupported => Some(muted(words.unsupported(), tokens)),
    })
    .children(errors(shell, host, tokens))
}

/// One label: its colour, its name, and the pencil that opens it. The row
/// is a fixed rail so the pencil fading in cannot move it.
fn row(
    shell: &Shell,
    host: &Host,
    label: &Label,
    enabled: bool,
    narrow: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let name = label.name.as_str().to_owned();
    let group = SharedString::from(format!("label-row-{name}"));
    let selector = group.to_string();
    // Paseo reveals the pencil on hover, and always on a phone.
    let revealed = narrow;
    let pencil_group = group.clone();
    let pencil = div()
        .id(ElementId::Name(format!("label-edit-{name}").into()))
        .debug_selector({
            let name = name.clone();
            move || format!("label-edit-{name}")
        })
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .child(glyph(Glyph::Pencil, PENCIL, neutral.foreground_muted))
        .when(!revealed, |pencil| {
            pencil
                .opacity(0.0)
                .group_hover(pencil_group, |style| style.opacity(1.0))
        })
        .when(enabled, |pencil| {
            let label = label.clone();
            let route = host.route.clone();
            let connection = host.connection.clone();
            pencil
                .cursor_pointer()
                .active(|style| style.opacity(PRESSED))
                .on_click(cx.listener(move |shell, _, window, cx| {
                    shell.labels_edit(&route, connection.as_ref(), &label, window, cx);
                }))
        });
    div()
        .id(ElementId::Name(group.clone()))
        .debug_selector(move || selector.clone())
        .group(group)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(S2))
        .min_h(px(ROW_MIN))
        .child(super::colors::mark(label.color, tokens).flex_none())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .type_(Type::Meta)
                .text_color(neutral.foreground)
                .child(name),
        )
        .child(pencil)
}

impl Shell {
    /// Paseo's `model.edit`: the row's host must still be the shown one on
    /// the same connection.
    fn labels_edit(
        &mut self,
        route: &vessel_app_view::machine_book::Name,
        connection: Option<&vessel_app_view::connection::ConnectionContext>,
        label: &Label,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.labels;
        if !state
            .selected_host()
            .is_some_and(|host| &host.route == route && host.connection.as_ref() == connection)
        {
            return;
        }
        state.host_list.close();
        match state.manager.edit(label.clone()) {
            Ok(()) => {
                state.name = super::fields::manager(self.words.labels().name(), window, cx);
                state.applied_editability = None;
                state.name.set_text(label.name.as_str(), window, cx);
                state.name.focus(window, cx);
            }
            Err(failure) => state.error = Some(failure),
        }
        self.settle_labels(cx);
        cx.notify();
    }
}

/// A save or a check under way (in place of Paseo's spinner in Save).
fn pending(shell: &Shell, tokens: &Tokens) -> Option<Div> {
    let words = shell.words.labels();
    match shell.areas.labels.manager.state() {
        ManagerState::Saving { .. } => Some(muted(words.pending(), tokens)),
        ManagerState::Inspecting { .. } => Some(muted(words.inspecting(), tokens)),
        ManagerState::Listing
        | ManagerState::Editing(_)
        | ManagerState::Confirming { .. }
        | ManagerState::Deleting { .. } => None,
    }
}

/// The confirmation's connection is not the shown one any more.
fn changed(shell: &Shell, host: &Host, tokens: &Tokens) -> Option<Div> {
    (shell.areas.labels.confirmation.as_ref() != host.connection.as_ref())
        .then(|| error(shell.words.labels().changed(), tokens))
}

/// Paseo's error line: the operation's error, else the host's load error.
fn errors(shell: &Shell, host: &Host, tokens: &Tokens) -> Option<Div> {
    let state = &shell.areas.labels;
    let failure = state
        .error
        .or_else(|| state.manager.error())
        .or(match host.state {
            HostState::Failed(failure) => Some(failure),
            HostState::Online
            | HostState::Offline
            | HostState::Loading
            | HostState::Unsupported => None,
        })?;
    Some(error(failure_text(failure, shell), tokens))
}

/// Paseo's `muted` line: 12, `foregroundMuted`.
fn muted(text: &'static str, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .child(text)
}

/// Paseo's `error` line: 12, `destructive`.
fn error(text: &'static str, tokens: &Tokens) -> Div {
    div()
        .debug_selector(|| "labels-error".to_owned())
        .type_(Type::Meta)
        .text_color(tokens.neutral.destructive)
        .child(text)
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
