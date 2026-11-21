//! Settings › Plugins' installations, as Paseo's catalog and `PluginRow`:
//! "Loading plugins…" (a small alert), the error with Retry, "No plugins
//! configured", or a card of rows; a row's id with its status badge, its
//! description (14, muted), its source (12, extra muted) and its error
//! (12, the danger colour) under it; its switch and "…" at the right. The
//! "…" opens Paseo's 220 menu (a sheet on a phone): Logs, Reload (only
//! while enabled; "Reloading…" while it reloads), Remove (danger, with the
//! trash glyph; "Removing…").

use std::rc::Rc;

use vessel_app_kit::alert::{Content as AlertContent, Size, Variant, alert};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::badge::{self, badge};
use vessel_app_kit::menu::{
    Content, ItemStatus, MenuHandlers, MenuItem, TriggerPresentation, TriggerProfile,
    TriggerSizing, menu_with, trigger_presented,
};
use vessel_app_kit::settings;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, ClickEvent, Context, Div, FluentBuilder as _, Hsla, InteractiveElement as _,
    IntoElement as _, MouseButton, MouseDownEvent, ParentElement as _, Pixels, Point, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonState, Tokens, Type, TypeExt as _, outline_button};
use vessel_app_view::plugins::{
    Action, Page, PageState, Tone, problem_words, source_line, status_tone, status_word,
};
use vessel_proto::plugins::{PluginId, PluginListItem};

use crate::shell::Shell;

/// Paseo's menu width (`DropdownMenuContent width={220}`).
const MENU_WIDTH: f32 = 220.0;
/// Paseo's row words: at least 160 wide before the controls wrap under.
const WORDS_BASIS: f32 = 160.0;

/// The menu's rows, in Paseo's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowAct {
    Logs,
    Reload,
    Remove,
}

const ROW_ACTS: [RowAct; 3] = [RowAct::Logs, RowAct::Reload, RowAct::Remove];

impl Shell {
    /// A row's "…" was pressed: its menu, under it.
    fn plugins_menu(
        &mut self,
        plugin: PluginId,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.plugins;
        let drawn = state
            .anchors
            .borrow()
            .get(&plugin)
            .and_then(|anchor| anchor.get());
        state.menu_for = Some(plugin);
        match drawn {
            Some(bounds) => state.menu.open_end(bounds, window, cx),
            None => state.menu.open(at, window, cx),
        }
        cx.notify();
    }

    /// The "…" activated by a click (a pointer's press already opened the
    /// menu on its way down; assistive technology and the keyboard click
    /// only): open the menu unless it is already open for this plugin, so
    /// the click that ends a press does not close it again.
    fn plugins_menu_activate(
        &mut self,
        plugin: PluginId,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &self.areas.settings.plugins;
        if !(state.menu.is_open() && state.menu_for.as_ref() == Some(&plugin)) {
            self.plugins_menu(plugin, at, window, cx);
        }
    }

    /// Row `index` of the open menu was chosen.
    fn plugins_menu_chose(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(plugin) = self.areas.settings.plugins.menu_for.take() else {
            return;
        };
        match ROW_ACTS.get(index) {
            Some(RowAct::Logs) => self.plugins_open_logs(&plugin, window, cx),
            Some(RowAct::Reload) => self.plugins_act(Action::Reload, &plugin, window, cx),
            Some(RowAct::Remove) => self.plugins_act(Action::Remove, &plugin, window, cx),
            None => {}
        }
    }
}

/// The installations, or why there are none to show.
pub(super) fn catalog(
    shell: &Shell,
    page: Option<&Page>,
    state: PageState,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.plugins();
    let loading = || {
        small_alert(
            SharedString::new_static(words.loading()),
            None,
            None,
            Variant::Default,
            tokens,
        )
    };
    let Some(page) = page else {
        return loading();
    };
    match state {
        PageState::Error => {
            let why = page
                .list_problem()
                .map(|problem| problem_words(words, problem));
            let retry = outline_button(
                "plugins-retry",
                SharedString::new_static(words.retry()),
                true,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, _window, cx| shell.plugins_retry(cx)));
            small_alert(
                SharedString::new_static(words.error_title()),
                why,
                Some(retry.into_any_element()),
                Variant::Error,
                tokens,
            )
        }
        PageState::Empty => settings::card(tokens)
            .child(
                div()
                    .p(px(spacing::S4))
                    .flex()
                    .justify_center()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(SharedString::new_static(words.empty())),
            )
            .into_any_element(),
        PageState::Ready => settings::card(tokens)
            .children(
                page.plugins()
                    .iter()
                    .enumerate()
                    .map(|(at, plugin)| row(shell, page, plugin, at == 0, tokens, compact, cx)),
            )
            .into_any_element(),
        PageState::Loading | PageState::Offline | PageState::Unsupported => loading(),
    }
}

/// Paseo's small alert with a title, an optional description and action.
fn small_alert(
    title: SharedString,
    description: Option<String>,
    action: Option<AnyElement>,
    variant: Variant,
    tokens: &Tokens,
) -> AnyElement {
    alert(
        AlertContent {
            title: Some(title),
            description: description.map(|said| div().child(said).into_any_element()),
            action,
        },
        variant,
        Size::Small,
        tokens,
    )
    .into_any_element()
}

/// One installation (Paseo's `PluginRow`).
fn row(
    shell: &Shell,
    page: &Page,
    plugin: &PluginListItem,
    first: bool,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.plugins();
    let neutral = tokens.neutral;
    let variant = match status_tone(plugin.status) {
        Tone::Ok => badge::Variant::Success,
        Tone::Danger => badge::Variant::Error,
        Tone::Neutral => badge::Variant::Muted,
    };
    let title = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S2))
        .child(
            div()
                .min_w_0()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(plugin.id.as_str().to_owned()),
        )
        .child(badge(
            SharedString::new_static(status_word(words, plugin.status)),
            variant,
            badge::Size::Small,
            None,
            tokens,
        ));
    let line = |face: Type, color: Hsla, text: String| {
        div()
            .mt(px(spacing::S1))
            .type_(face)
            .text_color(color)
            .child(text)
    };
    let content = div()
        .flex_1()
        .min_w_0()
        .when(compact, |content| content.flex_basis(px(WORDS_BASIS)))
        .flex()
        .flex_col()
        .child(title)
        .children(plugin.description.as_ref().map(|description| {
            line(
                Type::Body,
                neutral.foreground_muted,
                description.as_str().to_owned(),
            )
        }))
        .children(
            source_line(words, plugin)
                .map(|source| line(Type::Meta, neutral.foreground_extra_muted, source)),
        )
        .children(
            plugin
                .error
                .clone()
                .map(|error| line(Type::Meta, neutral.destructive, error)),
        );
    let controls = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .child(row_switch(shell, page, plugin, tokens, compact, cx))
        .child(trigger(shell, page, &plugin.id, tokens, compact, cx));
    settings::row_with_content(
        content.into_any_element(),
        Some(controls.into_any_element()),
        first,
        compact,
        tokens,
    )
}

/// A row's switch: its saved intent (Paseo's `toggle`).
fn row_switch(
    shell: &Shell,
    page: &Page,
    plugin: &PluginListItem,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.plugins();
    let state = &shell.areas.settings.plugins;
    let on = plugin.enabled;
    let ready = !page.row(&plugin.id).busy;
    let id = plugin.id.clone();
    let said = if on {
        words.switch_disable(id.as_str())
    } else {
        words.switch_enable(id.as_str())
    }
    .unwrap_or_else(|_unformatted| id.as_str().to_owned());
    let turn = state.row_turns.get(&id).copied().unwrap_or(0);
    let control = settings::switch(
        SharedString::from(format!("plugin-switch-{id}")),
        on,
        ready,
        compact,
        turn,
        tokens,
    )
    .role(gpui::Role::CheckBox)
    .aria_toggled(if on {
        gpui::Toggled::True
    } else {
        gpui::Toggled::False
    })
    .aria_label(said);
    if !ready {
        return control.into_any_element();
    }
    let action = if on { Action::Disable } else { Action::Enable };
    control
        .on_click(cx.listener(move |shell, _event, window, cx| {
            shell.plugins_act(action, &id, window, cx);
        }))
        .into_any_element()
}

/// A row's "…" (Paseo's `PluginActionsMenu` trigger), waiting while a
/// change is made.
fn trigger(
    shell: &Shell,
    page: &Page,
    plugin: &PluginId,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = &shell.areas.settings.plugins;
    let ready = !page.row(plugin).busy;
    let availability = if ready {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let open = state.menu.is_open() && state.menu_for.as_ref() == Some(plugin);
    let presentation = if open {
        TriggerPresentation::Open(availability)
    } else {
        TriggerPresentation::Closed(availability)
    };
    let sizing = if compact {
        TriggerSizing::Touch
    } else {
        TriggerSizing::Pointer
    };
    let label = menu_label(shell, plugin);
    let element = trigger_presented(
        SharedString::from(format!("plugin-menu-{plugin}")),
        &state.anchor(plugin),
        TriggerProfile::ProviderRow,
        sizing,
        presentation,
        tokens,
    )
    .role(gpui::Role::Button)
    .aria_label(label)
    .aria_expanded(open);
    if !ready {
        return element.into_any_element();
    }
    let pressed = plugin.clone();
    let clicked = plugin.clone();
    element
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |shell, event: &MouseDownEvent, window, cx| {
                shell.plugins_menu(pressed.clone(), event.position, window, cx);
            }),
        )
        // A click (assistive technology's Click action, or the end of a
        // pointer's press) opens it too, once (TRACKER 662's kebab).
        .on_click(cx.listener(move |shell, event: &ClickEvent, window, cx| {
            shell.plugins_menu_activate(clicked.clone(), event.position(), window, cx);
        }))
        .into_any_element()
}

/// "Actions for {id}": the trigger's name and the phone sheet's title.
fn menu_label(shell: &Shell, plugin: &PluginId) -> SharedString {
    shell
        .words
        .plugins()
        .menu(plugin.as_str())
        .unwrap_or_else(|_unformatted| plugin.as_str().to_owned())
        .into()
}

/// The open row's menu.
pub(super) fn menu(
    shell: &Shell,
    page: Option<&Page>,
    tokens: &Tokens,
    phone: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.plugins;
    let id = state.menu_for.as_ref()?;
    let page = page?;
    let plugin = page.plugins().iter().find(|plugin| &plugin.id == id)?;
    let words = shell.words.plugins();
    let look = page.row(id);
    let items: Vec<MenuItem> = ROW_ACTS
        .iter()
        .map(|act| {
            let (mut item, pending) = match act {
                RowAct::Logs => (MenuItem::new(SharedString::new_static(words.logs())), None),
                RowAct::Reload => (
                    MenuItem::new(SharedString::new_static(words.reload())),
                    (look.mine == Some(Action::Reload)).then_some(words.reloading()),
                ),
                RowAct::Remove => (
                    MenuItem::danger(SharedString::new_static(words.remove()))
                        .with_leading(Glyph::Trash2),
                    (look.mine == Some(Action::Remove)).then_some(words.removing()),
                ),
            };
            item.disabled = look.busy || (*act == RowAct::Reload && !plugin.enabled);
            if let Some(pending) = pending {
                item.status = ItemStatus::Pending(SharedString::new_static(pending));
            }
            item
        })
        .collect();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.settings.plugins.menu,
        chose: Rc::new(|shell: &mut Shell, index, window, cx| {
            shell.plugins_menu_chose(index, window, cx);
        }),
    };
    let content = Content::new("plugin-row-menu", &items)
        .with_width(px(MENU_WIDTH))
        .with_sheet(menu_label(shell, id));
    menu_with(content, &state.menu, &handlers, tokens, phone, cx)
}

#[cfg(test)]
mod tests;
