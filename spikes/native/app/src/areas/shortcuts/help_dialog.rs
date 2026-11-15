//! Searchable shortcut help reuses the command frame and key-badge renderer.
//! The dialog retains its own query/focus; it never creates another shortcut
//! registry, interceptor or preference write path.

mod rows;

use crate::shell::Shell;
use gpui::{FocusHandle, ScrollHandle};
use vessel_app_kit::command::{
    CommandAnimation, CommandFrame, CommandHandlers, CommandKey, CommandState,
    command_frame_animated,
};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _};

pub(super) struct HelpDialog {
    motion: CommandState,
    search: TextField,
    query: String,
    scroll: ScrollHandle,
    prior: Option<FocusHandle>,
}

impl HelpDialog {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            motion: CommandState::default(),
            search: TextField::with_change(
                "",
                |_, _, _, _| {},
                Shell::shortcuts_help_searched,
                window,
                cx,
            ),
            query: String::new(),
            scroll: ScrollHandle::new(),
            prior: None,
        }
    }

    pub(super) const fn is_open(&self) -> bool {
        self.motion.is_open()
    }
}

impl Shell {
    pub(crate) fn refresh_shortcut_words(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.refresh_shortcut_conflicts();
        self.areas.shortcuts.help.search.set_placeholder(
            self.words.shortcuts().search(),
            window,
            cx,
        );
    }

    /// Help, the palette and the shortcut all open this same retained dialog.
    pub(crate) fn open_shortcuts_help(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if f32::from(window.viewport_size().width) < vessel_app_kit::picker::COMPACT {
            return;
        }
        let help = &mut self.areas.shortcuts.help;
        if !help.is_open() {
            help.prior = window.focused(cx);
            help.query.clear();
            help.search.clear(window, cx);
            help.search
                .set_placeholder(self.words.shortcuts().search(), window, cx);
            help.motion.present();
        }
        help.search.focus(window, cx);
        cx.notify();
    }

    pub(super) fn toggle_shortcuts_help(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.shortcuts.help.is_open() {
            self.close_shortcuts_help(window, cx);
        } else {
            self.open_shortcuts_help(window, cx);
        }
    }

    fn close_shortcuts_help(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let help = &mut self.areas.shortcuts.help;
        help.motion.close();
        help.query.clear();
        help.search.clear(window, cx);
        if let Some(prior) = help.prior.take() {
            prior.focus(window, cx);
        } else {
            window.blur(cx);
        }
        cx.notify();
    }

    fn shortcuts_help_searched(&mut self, query: String, cx: &mut Context<'_, Self>) {
        self.areas.shortcuts.help.query = query;
        cx.notify();
    }

    fn shortcuts_help_key(
        &mut self,
        key: CommandKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        match key {
            CommandKey::Escape => self.close_shortcuts_help(window, cx),
            CommandKey::Up | CommandKey::Down => {
                let help = &self.areas.shortcuts.help;
                let old = help.scroll.offset();
                let amount = px(spacing::S8);
                let y = if key == CommandKey::Up {
                    old.y + amount
                } else {
                    old.y - amount
                };
                help.scroll
                    .set_offset(vessel_app_kit::ui::point(old.x, y.min(px(spacing::S0))));
                cx.notify();
            }
            CommandKey::Enter => self.areas.shortcuts.help.search.focus(window, cx),
        }
    }
}

pub(crate) fn render(
    shell: &mut Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if f32::from(window.viewport_size().width) < vessel_app_kit::picker::COMPACT {
        return None;
    }
    let motion = shell.areas.shortcuts.help.motion.sample(
        false,
        window.viewport_size().height,
        cx.reduce_motion(),
    );
    if motion.hidden {
        return None;
    }
    let list = rows::render(shell, tokens);
    let help = &shell.areas.shortcuts.help;
    let search = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Label)
                .child(shell.words.shortcuts().dialog_title()),
        )
        .child(help.search.bare(tokens, cx))
        .into_any_element();
    Some(command_frame_animated(
        CommandFrame {
            search,
            list,
            compact: false,
        },
        CommandAnimation {
            state: &help.motion,
            scroll: &help.scroll,
            motion,
        },
        CommandHandlers {
            on_key: Shell::shortcuts_help_key,
            close: Shell::close_shortcuts_help,
        },
        tokens,
        window,
        cx,
    ))
}
