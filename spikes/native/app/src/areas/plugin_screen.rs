//! A plugin's full-page screen (ADR 0115, "Surfaces"): Paseo's plugin screen
//! route (`packages/app/src/plugins/surface-screen.tsx:140-250`), with the
//! header (title, machine, Close) the app's and the body the plugin's
//! declared surface (`plugin_screen/controls.rs`).
//!
//! [Decision Log]
//! - Purpose: a sidebar row or a Command Center item opens a plugin screen
//!   by machine, plugin and screen id, as the page's
//!   `/plugin/$node/$plugin/$screen` does.
//! - `Route` is `Copy`, so the screen's address is this area's state, as
//!   the open session is the session area's.
//! - Paseo's states: a screen whose machine is not connected says the
//!   machine is offline; a screen no running plugin declares is
//!   unavailable; Close goes back to where the screen was opened from.
//! - Every device that reads the catalog draws screens. What calls a method
//!   is disabled without `may_invoke`, under one muted line (coordinator
//!   ruling on P3b, Q1).
//! - A text field is a kit field the screen keeps per control, made when
//!   the screen opens (a field needs the window); one the plugin declares
//!   later shows when the screen is opened again.

use std::cell::RefCell;
use std::collections::HashMap;

use vessel_app_kit::field::TextField;
use vessel_app_kit::menu::MenuState;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _, button_with_kind_at,
};
use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::plugins::{ScreenAddress, needs_admin};
use vessel_proto::plugins::{ContributionId, PluginId, Surface};

use crate::routes::Route;
use crate::shell::Shell;

mod controls;
mod inputs;
mod menu;

/// The open plugin screen, and what its controls keep.
pub(crate) struct ScreenArea {
    /// The screen shown, while the route is the plugin screen.
    pub(crate) open: Option<ScreenAddress>,
    /// The route to go back to.
    from: Route,
    /// The machine each plugin screen was last shown from (the sidebar's
    /// merged rows open it next).
    pub(crate) remembered: HashMap<(PluginId, ContributionId), MachineKey>,
    /// The open screen's text fields, by control id.
    fields: HashMap<ContributionId, TextField>,
    /// A toggle's or choice's value shown until its call answers, by
    /// control id; then the declaration wins again.
    pending: RefCell<HashMap<ContributionId, Pending>>,
    /// Which pending value a method call's answer clears.
    calls: HashMap<Ticket, ContributionId>,
    /// The popover buttons shown open (drawn in place under the button).
    unfolded: RefCell<Vec<ContributionId>>,
    /// The menu of a declared menu button.
    pub(crate) menu: MenuState,
    /// Which button's menu is open.
    pub(crate) menu_for: Option<ContributionId>,
}

/// A value shown while its call is out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Pending {
    /// A toggle's new state.
    Toggle(bool),
    /// A choice's new option.
    Choice(ContributionId),
}

impl ScreenArea {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: None,
            from: Route::Board,
            remembered: HashMap::new(),
            fields: HashMap::new(),
            pending: RefCell::default(),
            calls: HashMap::new(),
            unfolded: RefCell::default(),
            menu: MenuState::new(cx),
            menu_for: None,
        }
    }
}

impl Shell {
    /// Open `at`, from wherever the window is now.
    pub(crate) fn open_plugin_screen(
        &mut self,
        at: ScreenAddress,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.plugin_screen;
        if self.route != Route::Plugin {
            area.from = self.route;
        }
        let _shown = area
            .remembered
            .insert((at.plugin.clone(), at.screen.clone()), at.machine.clone());
        area.fields.clear();
        area.pending.borrow_mut().clear();
        area.unfolded.borrow_mut().clear();
        area.menu_for = None;
        area.open = Some(at);
        self.plugin_screen_fields(window, cx);
        self.go(Route::Plugin, window, cx);
    }

    /// Make the open screen's text fields.
    fn plugin_screen_fields(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(at) = self.areas.plugin_screen.open.clone() else {
            return;
        };
        let Some(found) = self.areas.plugin_catalogs.screen(&at) else {
            return;
        };
        let wanted = controls::text_fields(found.surface);
        for (id, method) in wanted {
            let control = id.clone();
            let field = TextField::new(
                "",
                move |shell: &mut Self, text, _window, cx| {
                    shell.plugin_screen_submit(&control, &method, text, cx);
                },
                window,
                cx,
            );
            let _made = self.areas.plugin_screen.fields.insert(id, field);
        }
    }

    /// Close the screen: back to where it was opened from.
    pub(crate) fn close_plugin_screen(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let from = self.areas.plugin_screen.from;
        self.areas.plugin_screen.open = None;
        self.go(from, window, cx);
    }

    /// A method call from the screen answered: its pending value goes.
    pub(crate) fn plugin_screen_settled(&mut self, ticket: Ticket) {
        let area = &mut self.areas.plugin_screen;
        if let Some(control) = area.calls.remove(&ticket) {
            let _cleared = area.pending.borrow_mut().remove(&control);
        }
    }
}

/// Draw the open screen.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.plugins();
    let area = &shell.areas.plugin_screen;
    let found = area.open.as_ref().and_then(|at| {
        shell
            .areas
            .plugin_catalogs
            .screen(at)
            .map(|found| (at, found))
    });
    let title: SharedString = found
        .map_or_else(
            || area.open.as_ref().map_or("", |at| at.plugin.as_str()),
            |(_, found)| found.title,
        )
        .to_owned()
        .into();
    let compact = vessel_app_kit::breakpoint::phone(window);
    let body = match found {
        None => muted(words.screen_unavailable(), tokens),
        Some((at, _)) if !connected(shell, &at.machine) => muted(words.screen_offline(), tokens),
        Some((at, found)) => {
            let scope = controls::Scope {
                at,
                installation: found.installation,
                may_invoke: found.may_invoke,
                compact,
            };
            surface(shell, &scope, found.surface, tokens, window, cx)
        }
    };
    let machine = area.open.as_ref().map(|at| at.machine.as_str().to_owned());
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(header(title, machine, compact, tokens, shell, cx))
        .child(
            div()
                .id("plugin-screen-body")
                .flex_1()
                .overflow_y_scroll()
                .p(px(spacing::S5))
                .child(
                    div()
                        .max_w(px(720.0))
                        .mx_auto()
                        .flex()
                        .flex_col()
                        .gap(px(spacing::S4))
                        .child(body),
                ),
        )
        .children(menu::render(shell, tokens, compact, cx))
        .into_any_element()
}

/// The body: the read-only line when this device cannot act, then the
/// declared controls.
fn surface(
    shell: &Shell,
    scope: &controls::Scope<'_>,
    surface: &Surface,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.plugins();
    let machine = scope.at.machine.as_str();
    let line = (!scope.may_invoke && needs_admin(surface)).then(|| {
        muted_owned(
            words
                .screen_read_only(machine)
                .unwrap_or_else(|_unformatted| machine.to_owned()),
            tokens,
        )
    });
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S4))
        .children(line)
        .children(controls::render(
            shell,
            scope,
            &surface.controls,
            tokens,
            window,
            cx,
        ))
        .into_any_element()
}

/// Paseo's header: the title, the machine under it, Close at the right.
fn header(
    title: SharedString,
    machine: Option<String>,
    compact: bool,
    tokens: &Tokens,
    shell: &Shell,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.plugins();
    let sizing = if compact {
        ControlSizing::Touch
    } else {
        ControlSizing::Pointer
    };
    let close = button_with_kind_at(
        "plugin-screen-close",
        SharedString::new_static(words.screen_close()),
        ButtonKind::Ghost,
        ButtonState::Enabled,
        sizing,
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| shell.close_plugin_screen(window, cx)));
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(spacing::S3))
        .px(px(spacing::S5))
        .py(px(spacing::S3))
        .border_b_1()
        .border_color(tokens.border_subtle)
        .child(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .child(
                    div()
                        .type_(Type::Title)
                        .text_color(tokens.text_primary)
                        .truncate()
                        .child(title),
                )
                .children(machine.map(|machine| {
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.text_tertiary)
                        .child(machine)
                })),
        )
        .child(close)
}

/// Whether `machine` is connected now.
fn connected(shell: &Shell, machine: &MachineKey) -> bool {
    shell
        .fleet
        .online()
        .any(|held| MachineKey::from_book(&held.name) == *machine)
}

fn muted(text: &'static str, tokens: &Tokens) -> AnyElement {
    muted_owned(text.to_owned(), tokens)
}

fn muted_owned(text: String, tokens: &Tokens) -> AnyElement {
    div()
        .type_(Type::Meta)
        .text_color(tokens.text_secondary)
        .child(text)
        .into_any_element()
}

/// The element id of control `id` drawn under `prefix`.
pub(crate) fn control_id(prefix: &str, id: &ContributionId) -> SharedString {
    format!("plugin-control-{prefix}-{id}").into()
}
