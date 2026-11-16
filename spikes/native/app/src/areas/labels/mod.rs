//! Workspace labels (ADR 0086, protocol 28): the manager on the adaptive
//! dialog, the workspace picker from a session row's menu, and each host's
//! catalog and registry followed for the rows. State and rules belong to
//! the view.

mod actions;
mod colors;
mod draw;
#[cfg(test)]
mod draw_tests;
mod fields;
mod follow;
mod form;
mod host_filter;
mod hosts;
mod picker;
mod surface;

pub(crate) use colors::chip;

use vessel_app_kit::ui::{Context, IntoElement as _, Window};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogState, SearchField, TextField, Tokens, button_with_kind,
};
use vessel_app_view::labels::{EditAccess, Failure, Host, HostState, Manager};

use crate::shell::Shell;

pub(crate) struct Labels {
    picker: Option<picker::WorkspacePicker>,
    picker_combo: vessel_app_kit::ComboboxState,
    /// Each connected, capable host's followed catalog and registry.
    follows: Vec<follow::Follow>,
    opening: Option<surface::Opening>,
    manager: Manager,
    hosts: Vec<Host>,
    route: Option<vessel_app_view::machine_book::Name>,
    confirmation: Option<vessel_app_view::connection::ConnectionContext>,
    pending: Option<(
        vessel_app_view::connection::ConnectionContext,
        vessel_app_view::labels::Ticket,
    )>,
    search: SearchField,
    name: TextField,
    dialog: DialogState,
    /// Paseo's `HostFilter` list and where its pill was last drawn.
    host_list: vessel_app_kit::ComboboxState,
    host_anchor: vessel_app_kit::select::Anchor,
    error: Option<Failure>,
    unresolved: Vec<vessel_app_view::machine_book::Name>,
    applied_editability: Option<EditAccess>,
    tickets: vessel_app_view::labels::Tickets,
}

impl Labels {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            picker: None,
            picker_combo: vessel_app_kit::ComboboxState::new(window, cx),
            follows: Vec::new(),
            opening: None,
            manager: Manager::default(),
            hosts: Vec::new(),
            route: None,
            pending: None,
            confirmation: None,
            search: fields::search("", window, cx),
            name: fields::manager("", window, cx),
            dialog: DialogState::new(cx),
            host_list: vessel_app_kit::ComboboxState::new(window, cx),
            host_anchor: vessel_app_kit::select::Anchor::default(),
            error: None,
            unresolved: Vec::new(),
            applied_editability: None,
            tickets: vessel_app_view::labels::Tickets::default(),
        }
    }
}

/// One registration in the sidebar, without another navigation renderer.
pub(crate) fn entry(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> impl vessel_app_kit::ui::IntoElement {
    button_with_kind(
        "labels-open",
        shell.words.labels().title(),
        ButtonKind::Ghost,
        ButtonState::Enabled,
        tokens,
    )
    .on_click(cx.listener(|shell, _, window, cx| shell.labels_open(window, cx)))
    .into_any_element()
}

use vessel_app_kit::ui::StatefulInteractiveElement as _;

impl Shell {
    /// Refresh retained controls without replacing a person's text or selection.
    pub(crate) fn refresh_label_words(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        state
            .search
            .set_placeholder(self.words.labels().search(), window, cx);
        state
            .name
            .set_placeholder(self.words.labels().name(), window, cx);
    }

    fn labels_open(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.labels.picker = None;
        let state = &mut self.areas.labels;
        if state.opening.is_some() {
            state.dialog.focus(window, cx);
            cx.notify();
            return;
        }
        let opening = match state.tickets.issue() {
            Ok(ticket) => surface::Opening::new(ticket),
            Err(failure) => {
                state.error = Some(failure);
                cx.notify();
                return;
            }
        };
        // Each reopening owns its search input; a retained old event cannot
        // restore a cleared query after close. Busy drafts keep their query.
        state.search = fields::search(self.words.labels().search(), window, cx);
        state.search.set_text(state.manager.query(), window, cx);
        if state.manager.state().busy() {
            state.opening = Some(opening);
            state.dialog.focus(window, cx);
            cx.notify();
            return;
        }
        if let Err(failure) = state.manager.reset() {
            state.error = Some(failure);
            return;
        }
        state.error = None;
        hosts::bind(state, &self.fleet);
        state.search.clear(window, cx);
        state
            .search
            .set_placeholder(self.words.labels().search(), window, cx);
        state
            .name
            .set_placeholder(self.words.labels().name(), window, cx);
        state.opening = Some(opening);
        // Paseo's manager passes no `autoFocus` to its header search: the
        // dialog takes focus (Escape, Tab) and the field waits for a click.
        state.dialog.focus(window, cx);
        cx.notify();
    }

    fn labels_close(
        &mut self,
        expected: surface::Dismiss,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.labels;
        match expected.decide(state.opening, state.manager.state()) {
            surface::Outcome::Stale => return,
            surface::Outcome::CancelConfirmation => {
                let _cancelled = state.manager.cancel_delete();
                state.name.focus(window, cx);
            }
            surface::Outcome::Close => {
                state.opening = None;
                state.host_list.close();
                state.dialog.dismissed(window, cx);
                // Accepted writes keep their owner even while hidden.
                if !state.manager.state().busy() {
                    match state.manager.reset() {
                        Ok(()) => state.error = None,
                        Err(failure) => state.error = Some(failure),
                    }
                }
            }
        }
        self.settle_labels(cx);
        cx.notify();
    }

    /// Apply the shared editor policy only on transitions; no render-notify loop.
    pub(crate) fn settle_labels(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        if state.opening.is_none() {
            return;
        }
        let host = state.selected_host();
        let access = state
            .manager
            .edit_access(host.map_or(HostState::Offline, |host| host.state));
        if state.applied_editability != Some(access) {
            state.name.set_editability(
                match access {
                    EditAccess::Editable => vessel_app_kit::Editability::Editable,
                    EditAccess::ReadOnly => vessel_app_kit::Editability::ReadOnly,
                },
                cx,
            );
            state.applied_editability = Some(access);
        }
    }

    /// Fleet metadata may change while the manager is already open.
    pub(crate) fn labels_fleet(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        hosts::bind(state, &self.fleet);
        self.labels_follow(cx);
        self.settle_labels(cx);
        self.labels_picker_settle(cx);
    }
}

/// The picker or the manager, whichever is open. The shell draws it just
/// before the composer's overlay, which ends with the frame log that times
/// the whole draw.
pub(crate) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<vessel_app_kit::ui::AnyElement> {
    if shell.areas.labels.picker.is_some() {
        return picker::overlay(shell, tokens, window, cx);
    }
    draw::overlay(shell, tokens, window, cx)
}
