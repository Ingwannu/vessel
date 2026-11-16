//! Searchable shortcut help reuses the command frame and key-badge renderer.
//! The dialog retains its own query/focus; it never creates another shortcut
//! registry, interceptor or preference write path.

mod rows;

use crate::shell::Shell;
use gpui::ScrollHandle;
use vessel_app_kit::command::{
    CommandAnimation, CommandFrame, CommandHandlers, CommandKey, CommandState,
    command_frame_animated,
};
use vessel_app_kit::focus::ReturnFocus;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _};

pub(super) struct HelpDialog {
    motion: CommandState,
    search: TextField,
    pub(super) query: String,
    scroll: ScrollHandle,
    return_focus: ReturnFocus,
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
            return_focus: ReturnFocus::default(),
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
        if !super::available(window) {
            return;
        }
        self.close_palette(window, cx);
        super::modal_opened(self);
        let help = &mut self.areas.shortcuts.help;
        if !help.is_open() {
            help.return_focus.remember(window, cx);
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

    pub(crate) fn close_shortcuts_help(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let help = &mut self.areas.shortcuts.help;
        if !help.is_open() {
            return;
        }
        help.motion.close();
        help.query.clear();
        help.search.clear(window, cx);
        let owner = help.search.focus_handle(cx);
        help.return_focus.restore_if_owned(&owner, window, cx);
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
                let top = help.scroll.top_item();
                let next = if key == CommandKey::Up {
                    top.saturating_sub(1)
                } else {
                    top.saturating_add(1)
                };
                help.scroll.scroll_to_top_of_item(next);
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
    if !super::available(window) {
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
