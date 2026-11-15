//! Label manager on the existing adaptive dialog. State/rules belong to view.
//! The skeleton exposes unsupported nodes honestly until ADR 0086 is wired.

mod colors;
mod draw;
mod hosts;

use vessel_app_kit::ui::{Context, IntoElement as _, Window};
use vessel_app_kit::{ButtonKind, ButtonState, DialogState, TextField, Tokens, button_with_kind};
use vessel_app_view::labels::{EditAccess, Failure, Host, HostState, Manager};

use crate::shell::Shell;

pub(crate) struct Labels {
    open: bool,
    manager: Manager,
    hosts: Vec<Host>,
    search: TextField,
    name: TextField,
    dialog: DialogState,
    error: Option<Failure>,
    color_turn: colors::ColorTurn,
    unresolved: Vec<vessel_app_view::machine_book::Name>,
    applied_editability: Option<EditAccess>,
}

impl Labels {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: false,
            manager: Manager::default(),
            hosts: Vec::new(),
            search: TextField::with_change(
                "",
                |_shell: &mut Shell, _text, _window, _cx| {},
                |shell, query, cx| {
                    shell.areas.labels.manager.search(&query);
                    cx.notify();
                },
                window,
                cx,
            ),
            name: TextField::with_change(
                "",
                |_shell: &mut Shell, _text, _window, _cx| {},
                |shell, text, cx| {
                    if let Some(draft) = shell.areas.labels.manager.draft_mut() {
                        draft.set_text(&text);
                    }
                    cx.notify();
                },
                window,
                cx,
            ),
            dialog: DialogState::new(cx),
            error: None,
            color_turn: colors::ColorTurn::default(),
            unresolved: Vec::new(),
            applied_editability: None,
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
        let state = &mut self.areas.labels;
        if state.manager.state().busy() {
            state.open = true;
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
        state.manager.sync_hosts(&state.hosts);
        state.search.clear(window, cx);
        state
            .search
            .set_placeholder(self.words.labels().search(), window, cx);
        state
            .name
            .set_placeholder(self.words.labels().name(), window, cx);
        state.open = true;
        state.dialog.focus(window, cx);
        state.search.focus(window, cx);
        cx.notify();
    }

    fn labels_close(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        state.open = false;
        // Surface lifetime is independent of a committed/in-flight mutation.
        if !state.manager.state().busy() {
            match state.manager.reset() {
                Ok(()) => state.error = None,
                Err(failure) => state.error = Some(failure),
            }
        }
        cx.notify();
    }

    /// Apply the shared editor policy only on transitions; no render-notify loop.
    pub(crate) fn settle_labels(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        if !state.open {
            return;
        }
        let host = state
            .manager
            .selected()
            .and_then(|key| state.hosts.iter().find(|host| &host.key == key));
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
        state.manager.sync_hosts(&state.hosts);
        self.settle_labels(cx);
    }
}

pub(crate) use draw::overlay;
