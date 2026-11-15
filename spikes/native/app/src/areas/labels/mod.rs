//! Label manager on the existing adaptive dialog. State/rules belong to view.
//! The skeleton exposes unsupported nodes honestly until ADR 0086 is wired.

mod draw;

use vessel_app_kit::ui::{Context, IntoElement as _, Window};
use vessel_app_kit::{ButtonKind, ButtonState, DialogState, TextField, Tokens, button_with_kind};
use vessel_app_view::labels::{Catalog, Failure, Host, HostKey, HostState, Manager};

use crate::shell::Shell;

pub(crate) struct Labels {
    pub(super) open: bool,
    pub(super) manager: Manager,
    pub(super) hosts: Vec<Host>,
    pub(super) search: TextField,
    pub(super) name: TextField,
    pub(super) dialog: DialogState,
    pub(super) error: Option<Failure>,
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

use vessel_app_kit::ui::InteractiveElement as _;

impl Shell {
    fn labels_open(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        if let Err(failure) = state.manager.reset() {
            state.error = Some(failure);
            return;
        }
        state.error = None;
        state.hosts.clear();
        for machine in &self.fleet.machines {
            let key = match HostKey::parse(&machine.name) {
                Ok(key) => key,
                Err(_invalid_book_name) => {
                    state.error = Some(Failure::Load);
                    continue;
                }
            };
            let availability = match machine.reach {
                vessel_app_view::Reach::Online => HostState::Unsupported,
                vessel_app_view::Reach::Unreachable(_) => HostState::Offline,
            };
            state.hosts.push(Host {
                key,
                state: availability,
                catalog: Catalog::default(),
            });
        }
        if let Some(host) = state.hosts.first() {
            if let Err(failure) = state.manager.select(host.key.clone()) {
                state.error = Some(failure);
            }
        }
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

    pub(super) fn labels_close(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        match state.manager.reset() {
            Ok(()) => {
                state.open = false;
                state.error = None;
            }
            Err(failure) => state.error = Some(failure),
        }
        cx.notify();
    }
}

pub(crate) use draw::overlay;
