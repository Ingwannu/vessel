//! Settings › Plugins' dialogs: Paseo's remove question (`confirmDialog`:
//! "Remove {id}?", the source directory is not deleted; Cancel then a
//! destructive Remove, as Settings › Devices' revoke question orders them)
//! and its `PluginLogsSheet` ("Logs: {id}" with Refresh in its header;
//! "Loading logs…", the error with Refresh, "No plugin output yet", or each
//! entry on `surface2`, 12 around, radius 6, 8 apart: its head 12 mono,
//! muted or the danger colour for stderr, over its message 14 mono).

use vessel_app_kit::alert::{Content, Size, Variant, alert};
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogContent, DialogSize, Tokens, Type, TypeExt as _,
    button_with_kind, dialog, outline_button,
};
use vessel_app_view::plugins::{LogsView, Page, log_head, problem_words};
use vessel_proto::plugins::{LogEntry, LogStream, PluginId};

use crate::shell::Shell;

impl Shell {
    /// The remove question was cancelled or dismissed: "Kept {id}".
    fn plugins_keep(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.plugins;
        if let Some(page) = state.page_mut() {
            page.keep();
        }
        state.remove_dialog.dismissed(window, cx);
        cx.notify();
    }

    /// The remove question was confirmed.
    fn plugins_remove(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.plugin_step(Page::remove, cx);
        self.areas
            .settings
            .plugins
            .remove_dialog
            .dismissed(window, cx);
    }

    /// Open `plugin`'s logs (the menu's Logs).
    pub(super) fn plugins_open_logs(
        &mut self,
        plugin: &PluginId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.plugin_step(|page, tickets| page.open_logs(tickets, plugin), cx);
        self.areas.settings.plugins.logs_dialog.focus(window, cx);
    }

    /// Read the logs again.
    fn plugins_refresh_logs(&mut self, cx: &mut Context<'_, Self>) {
        self.plugin_step(Page::refresh_logs, cx);
    }

    /// Close the logs, giving focus back to what opened them.
    fn plugins_close_logs(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.plugins;
        if let Some(page) = state.page_mut() {
            page.close_logs();
        }
        state.logs_dialog.dismissed(window, cx);
        cx.notify();
    }
}

/// Whether a dialog takes the phone's form (Settings › Devices' rule).
fn compact(window: &Window) -> bool {
    window.viewport_size().width < px(vessel_app_kit::picker::COMPACT)
}

/// The remove question, while it is asked.
pub(super) fn remove(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.plugins;
    let plugin = state.page()?.removing()?;
    let words = shell.words.plugins();
    let title = words
        .remove_title(plugin.as_str())
        .unwrap_or_else(|_unformatted| plugin.as_str().to_owned());
    let compact = compact(window);
    let touch = |button: Stateful<Div>| {
        button.when(compact, |button| {
            button.min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
        })
    };
    let content = DialogContent {
        title: title.into(),
        body: div()
            .child(SharedString::new_static(words.remove_message()))
            .into_any_element(),
        footer: vec![
            touch(outline_button(
                "plugin-remove-cancel",
                SharedString::new_static(words.cancel()),
                true,
                tokens,
            ))
            .on_click(cx.listener(|shell, _event, window, cx| shell.plugins_keep(window, cx)))
            .into_any_element(),
            touch(button_with_kind(
                "plugin-remove-confirm",
                SharedString::new_static(words.remove()),
                ButtonKind::Destructive,
                ButtonState::Enabled,
                tokens,
            ))
            .on_click(cx.listener(|shell, _event, window, cx| shell.plugins_remove(window, cx)))
            .into_any_element(),
        ],
        size: DialogSize::Normal,
    };
    Some(dialog(
        &state.remove_dialog,
        content,
        tokens,
        window,
        compact,
        Shell::plugins_keep,
        cx,
    ))
}

/// The logs dialog, while it is open.
pub(super) fn logs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.plugins;
    let logs = state.page()?.logs()?;
    let words = shell.words.plugins();
    let refreshing = logs.refreshing();
    let refresh = |id: &'static str, cx: &mut Context<'_, Shell>| {
        let label = if refreshing {
            words.logs_refreshing()
        } else {
            words.logs_refresh()
        };
        outline_button(id, SharedString::new_static(label), !refreshing, tokens)
            .when(!refreshing, |button| {
                button.on_click(cx.listener(|shell, _event, _window, cx| {
                    shell.plugins_refresh_logs(cx);
                }))
            })
            .into_any_element()
    };
    let muted_state = |text: &'static str| {
        div()
            .py(px(spacing::S6))
            .flex()
            .justify_center()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(SharedString::new_static(text))
            .into_any_element()
    };
    let body = match logs.view() {
        LogsView::Loading => muted_state(words.logs_loading()),
        LogsView::Failed(problem) => alert(
            Content {
                title: Some(SharedString::new_static(words.logs_error_title())),
                description: Some(
                    div()
                        .child(problem_words(words, problem))
                        .into_any_element(),
                ),
                action: Some(refresh("plugin-logs-retry", cx)),
            },
            Variant::Error,
            Size::Small,
            tokens,
        )
        .into_any_element(),
        LogsView::Empty => muted_state(words.logs_empty()),
        LogsView::Entries(entries) => div()
            .flex()
            .flex_col()
            .gap(px(spacing::S2))
            .children(entries.iter().map(|entry| entry_view(entry, tokens)))
            .into_any_element(),
    };
    let plugin = logs.plugin().as_str();
    let content = DialogContent {
        title: words
            .logs_title(plugin)
            .unwrap_or_else(|_unformatted| plugin.to_owned())
            .into(),
        body,
        footer: Vec::new(),
        size: DialogSize::Wide,
    };
    let actions = refresh("plugin-logs-refresh", cx);
    Some(dialog(
        &state.logs_dialog,
        content.with_header_actions(actions),
        tokens,
        window,
        compact(window),
        Shell::plugins_close_logs,
        cx,
    ))
}

/// One log entry: its head over its message.
fn entry_view(entry: &LogEntry, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    let head = if entry.stream == LogStream::Stderr {
        neutral.destructive
    } else {
        neutral.foreground_muted
    };
    div()
        .p(px(spacing::S3))
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .bg(neutral.surface2)
        .rounded(px(radius::MD))
        .child(
            div()
                .type_(Type::Mono)
                .text_color(head)
                .child(log_head(entry)),
        )
        .child(
            div()
                .type_(Type::MonoBody)
                .text_color(neutral.foreground)
                .child(entry.message.as_str().to_owned()),
        )
        .into_any_element()
}
