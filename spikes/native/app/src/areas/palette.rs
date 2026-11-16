//! One command center over source-owned contributions and existing built-ins.
//! Mod+K or the sidebar opens it; local arrows/Enter/Escape belong to the kit
//! frame before global rebinding. Source callbacks retain typed owner targets,
//! query ranking stays in view, and actual close uses shared weak return focus.
//! Legacy Settings/host/theme destinations remain searchable extensions;
//! missing feature-owner routes are never replaced with a different screen.

mod commands;
mod entry;
mod git_source;
mod layout_source;
mod messages;
mod navigation;
mod root;
mod rows;
mod snapshot;
pub(crate) mod source;
#[cfg(test)]
mod tests;

pub(crate) use entry::{sidebar_row, title_trigger};

use snapshot::Activate;

use gpui::ScrollHandle;
use vessel_app_kit::command::{
    CommandAnimation, CommandFrame, CommandHandlers, CommandKey, CommandState,
    command_frame_animated,
};
use vessel_app_kit::focus::ReturnFocus;
use vessel_app_kit::picker;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens};
use vessel_app_view::palette::{
    Direction, MessageCursor, PaletteCursor, PaletteEntry, PaletteMode, PaletteTarget,
    palette_entries,
};
use vessel_app_view::prefs::{PreferenceChange, Theme};
use vessel_app_view::settings::Section;

use super::session::Open;
use super::settings;
use crate::routes::Route;
use crate::shell::Shell;

/// One entry, with what the app's Settings and themes are.
type Entry = PaletteEntry<Section, Theme>;

/// What choosing an entry does.
type Target = PaletteTarget<Section, Theme>;

/// The palette's state.
pub(crate) struct PaletteArea {
    /// Persistent shared modal/snap-sheet motion.
    motion: CommandState,
    /// Its search.
    search: TextField,
    /// What the search holds.
    query: String,
    /// The row the keyboard is on, by its entry's id; the first listed
    /// when `None` or no longer listed.
    cursor: PaletteCursor<snapshot::Identity>,
    /// One window search survives replacement of any conversation resident.
    pub(crate) messages_search: crate::areas::conversation::search::Searching,
    sources: vessel_app_view::palette::registration::Registry<source::Command>,
    page: Option<vessel_app_view::palette::pages::ChoiceLocation>,
    root_help: Option<vessel_app_view::palette::registration::Owner>,
    keyboard_available: bool,
    git: git_source::State,
    layout: layout_source::State,
    /// Message selection has record identity rather than a command id.
    messages: MessageCursor,
    /// Commands remain the normal entry; transcript search is explicit.
    mode: PaletteMode,
    /// A result can disappear between the query and selection.
    unavailable: Option<messages::Unavailable>,
    /// The list's scroll, to keep the active row in sight.
    scroll: ScrollHandle,
    /// What held the keys before it opened.
    return_focus: ReturnFocus,
}

impl PaletteArea {
    /// Whether it is open for input (a closing phone sheet may still be
    /// drawn while this is false).
    pub(crate) const fn is_open(&self) -> bool {
        self.motion.is_open()
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            motion: CommandState::default(),
            search: TextField::with_change(
                "",
                |_, _, _, _| {},
                Shell::palette_searched,
                window,
                cx,
            ),
            query: String::new(),
            cursor: PaletteCursor::default(),
            messages_search: crate::areas::conversation::search::Searching::default(),
            sources: vessel_app_view::palette::registration::Registry::default(),
            page: None,
            root_help: None,
            keyboard_available: false,
            git: git_source::State::default(),
            layout: layout_source::State::default(),
            messages: MessageCursor::default(),
            mode: PaletteMode::default(),
            unavailable: None,
            scroll: ScrollHandle::new(),
            return_focus: ReturnFocus::default(),
        }
    }
}

impl Shell {
    pub(crate) fn refresh_command_sources(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.palette.page.take().is_some() {
            self.areas.palette.cursor.reset();
        }
        source::install_help(self, window, cx);
    }

    /// The toggle uses the same focus-restoring close path as Escape.
    pub(crate) fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.palette.is_open() {
            self.close_palette(window, cx);
        } else {
            self.open_palette(window, cx);
        }
    }

    /// Open the palette with an empty search holding the keys; when it is
    /// open already, give its search the keys again.
    pub(crate) fn open_palette(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.close_shortcuts_help(window, cx);
        crate::areas::shortcuts::modal_opened(self);
        self.areas.palette.keyboard_available = crate::areas::shortcuts::available(window);
        let placeholder = self.t.lang.palette().placeholder;
        let palette = &mut self.areas.palette;
        if !palette.motion.is_open() {
            palette.return_focus.remember(window, cx);
            palette.motion.present();
            palette.scroll.set_offset(vessel_app_kit::ui::point(
                px(vessel_app_kit::theme::spacing::S0),
                px(vessel_app_kit::theme::spacing::S0),
            ));
            palette.query.clear();
            palette.cursor.reset();
            palette.page = None;
            palette.messages.reset();
            palette.mode = PaletteMode::default();
            palette.unavailable = None;
            palette.search.set_placeholder(placeholder, window, cx);
            palette.search.clear(window, cx);
        }
        palette.search.focus(window, cx);
        source::install_help(self, window, cx);
        cx.notify();
    }

    /// Close the palette; the keys go back where they were.
    pub(crate) fn close_palette(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        git_source::close(self);
        layout_source::close(self);
        if let Some(owner) = self.areas.palette.root_help.take() {
            self.remove_command_source(&owner, cx);
        }
        let palette = &mut self.areas.palette;
        if !palette.motion.is_open() {
            return;
        }
        palette.motion.close();
        palette.page = None;
        palette.query.clear();
        palette.search.clear(window, cx);
        palette.cursor.reset();
        palette.messages.reset();
        palette.unavailable = None;
        let owner = palette.search.focus_handle(cx);
        palette.return_focus.restore_if_owned(&owner, window, cx);
        self.message_search_query("", cx);
        cx.notify();
    }

    fn palette_searched(&mut self, text: String, cx: &mut Context<'_, Self>) {
        if !self.areas.palette.is_open() {
            return;
        }
        self.areas.palette.query = text;
        self.areas.palette.cursor.reset();
        self.areas.palette.messages.reset();
        self.areas.palette.unavailable = None;
        if self.areas.palette.mode == PaletteMode::Messages {
            let query = self.areas.palette.query.clone();
            self.areas
                .palette
                .scroll
                .set_offset(vessel_app_kit::ui::point(
                    px(vessel_app_kit::theme::spacing::S0),
                    px(vessel_app_kit::theme::spacing::S0),
                ));
            self.message_search_query(&query, cx);
            return;
        }
        // A query reveals its highest-ranked row once. The next render/key
        // refreshes owner sources with the actual current Window.
        git_source::refresh(self);
        let snapshot = snapshot::project(self);
        let palette = &mut self.areas.palette;
        if let Some(at) = palette
            .cursor
            .resolved_identity(&snapshot.ids())
            .and_then(|id| snapshot.child_at(id))
        {
            palette.scroll.scroll_to_item(at);
        }
        cx.notify();
    }

    /// Everything the palette can list now.
    fn palette_all(&self) -> Vec<Entry> {
        let words = self.t.lang.settings();
        let sections: Vec<(Section, &str)> = Section::ALL
            .into_iter()
            .filter(|section| settings::drawn(*section))
            .map(|section| (section, words.section(section, self.words)))
            .collect();
        let selected = if self.route == Route::Draft {
            self.areas.draft.machine.clone()
        } else {
            self.machine_in_view()
        };
        let mut all = palette_entries(
            &self.fleet,
            selected.as_deref(),
            &sections,
            Theme::ALL.map(|theme| (theme, words.theme(theme))),
            self.t.lang,
        );
        if !settings::drawn(Section::Overview) {
            all.retain(|entry| !matches!(entry.target, PaletteTarget::Machine(_)));
        }

        all
    }

    fn palette_key(&mut self, key: CommandKey, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.palette.mode == PaletteMode::Messages {
            self.message_palette_key(key, window, cx);
            return;
        }
        match key {
            CommandKey::Up | CommandKey::Down => {
                source::refresh_dynamic(self, window);
                let snapshot = snapshot::project(self);
                let palette = &mut self.areas.palette;
                let direction = if key == CommandKey::Down {
                    Direction::Next
                } else {
                    Direction::Previous
                };
                if let Some(at) = palette
                    .cursor
                    .advance_identity(&snapshot.ids(), direction)
                    .and_then(|id| snapshot.child_at(id))
                {
                    palette.scroll.scroll_to_item(at);
                }
                cx.notify();
            }
            CommandKey::Enter => {
                source::refresh_dynamic(self, window);
                let snapshot = snapshot::project(self);
                let active = self.areas.palette.cursor.resolved_identity(&snapshot.ids());
                let target = active
                    .and_then(|id| snapshot.row(id))
                    .map(|row| row.activate.clone());
                if let Some(target) = target {
                    self.activate_command(target, window, cx);
                }
            }
            CommandKey::Escape => self.close_palette(window, cx),
        }
    }

    fn palette_back(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.palette.page = self
            .areas
            .palette
            .page
            .as_ref()
            .and_then(vessel_app_view::palette::pages::ChoiceLocation::parent);
        self.areas.palette.query.clear();
        self.areas.palette.search.clear(window, cx);
        self.areas.palette.cursor.reset();
        self.areas.palette.search.focus(window, cx);
        cx.notify();
    }

    fn activate_command(
        &mut self,
        target: Activate,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        source::refresh_dynamic(self, window);
        match target {
            Activate::Legacy(target) => self.run_palette(target, window, cx),
            Activate::Page(page) => self.palette_page(page, window, cx),
            Activate::Root(action) => self.run_command_action(action, window, cx),
            Activate::Registered(id, owner) => {
                let action = self
                    .areas
                    .palette
                    .sources
                    .resolve(&id, &owner)
                    .map(|entry| entry.target.action.clone());
                if let Some(action) = action {
                    self.run_command_action(action, window, cx);
                } else {
                    self.command_unavailable(cx);
                }
            }
        }
    }

    fn palette_page(
        &mut self,
        page: vessel_app_view::palette::pages::ChoiceLocation,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.palette.page = Some(page);
        self.areas.palette.query.clear();
        self.areas.palette.search.clear(window, cx);
        self.areas.palette.cursor.reset();
        self.areas.palette.search.focus(window, cx);
        cx.notify();
    }

    fn run_command_action(
        &mut self,
        action: source::CommandAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        match action {
            source::CommandAction::Shortcut(action) => {
                self.close_palette(window, cx);
                if !self.dispatch_shortcut_command(action, window, cx) {
                    self.command_unavailable(cx);
                }
            }
            source::CommandAction::Page(group) => self.palette_page(
                vessel_app_view::palette::pages::ChoiceLocation::group(group),
                window,
                cx,
            ),
            source::CommandAction::Legacy(target) => self.run_palette(target, window, cx),
            source::CommandAction::Callback(run) => {
                self.close_palette(window, cx);
                if !run(self, window, cx) {
                    self.command_unavailable(cx);
                }
            }
        }
    }

    fn command_unavailable(&mut self, cx: &mut Context<'_, Self>) {
        self.toast(
            self.words.shortcuts().command_unavailable().to_owned(),
            vessel_app_view::toasts::ToastTone::Error,
            cx,
        );
    }

    /// Close the palette and do what `target` says.
    fn run_palette(&mut self, target: Target, window: &mut Window, cx: &mut Context<'_, Self>) {
        if !matches!(target, PaletteTarget::Messages) {
            self.close_palette(window, cx);
        }
        match target {
            PaletteTarget::Messages => {
                self.palette_mode(PaletteMode::Messages, window, cx);
                self.areas.palette.search.clear(window, cx);
                self.palette_searched(String::new(), cx);
            }
            // Retained legacy rows cannot bypass A1's admitted opener even
            // after the speculative creation row disappears from projection.
            PaletteTarget::NewSession(_) => self.command_unavailable(cx),
            PaletteTarget::AddMachine => self.go(Route::Pair, window, cx),
            PaletteTarget::Board => self.go(Route::Board, window, cx),
            PaletteTarget::Settings(section) => self.open_settings(Some(section), window, cx),
            PaletteTarget::Machine(_) => {
                self.open_settings(Some(Section::Overview), window, cx);
            }
            PaletteTarget::Session {
                machine, session, ..
            } => {
                let open = self
                    .fleet
                    .machines
                    .iter()
                    .filter(|known| known.name == machine)
                    .flat_map(|known| &known.sessions)
                    .find(|card| card.session == session)
                    .map(Open::of);
                if let Some(open) = open {
                    self.open_session(open, window, cx);
                }
            }
            PaletteTarget::Theme(theme) => {
                self.prefer(&PreferenceChange::Theme(theme), window, cx);
            }
        }
    }
}

/// The palette, while open: over everything, at the window's top (a sheet
/// from the bottom at a phone's width).
pub(crate) fn render(
    shell: &mut Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    shell.areas.palette.keyboard_available = crate::areas::shortcuts::available(window);
    let compact = f32::from(window.viewport_size().width) < picker::COMPACT;
    let motion = shell.areas.palette.motion.sample(
        compact,
        window.viewport_size().height,
        cx.reduce_motion(),
    );
    if motion.hidden {
        return None;
    }
    let list = match shell.areas.palette.mode {
        PaletteMode::Commands => commands::render(shell, tokens, window, cx),
        PaletteMode::Messages => messages::render(shell, tokens, window, cx),
    };
    let search = div()
        .flex()
        .flex_col()
        .children(navigation::scope(shell, tokens, cx))
        .child(messages::scope(shell, tokens, window, cx))
        .child(shell.areas.palette.search.bare(tokens, cx))
        .into_any_element();
    let palette = &shell.areas.palette;
    Some(command_frame_animated(
        CommandFrame {
            search,
            list,
            compact,
        },
        CommandAnimation {
            state: &palette.motion,
            scroll: &palette.scroll,
            motion,
        },
        CommandHandlers {
            on_key: Shell::palette_key,
            close: Shell::close_palette,
        },
        tokens,
        window,
        cx,
    ))
}
