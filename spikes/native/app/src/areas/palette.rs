//! The command palette (DESIGN.md §2.2; the page's `features/palette`),
//! drawn as Paseo's command center (`kit::command`): its shortcut (Mod+K,
//! or `/` when not typing) or the sidebar's search row opens it over the
//! page; typing filters what `view::palette` lists, the arrows move the
//! active row, Enter or a click runs it, Escape or a press outside closes
//! it, and the keys go back where they were.
//!
//! The page lists the machines' Settings detail and every Settings
//! section; here only the sections Settings draws are listed (a section
//! still a placeholder would be a dead end), and machines only once
//! Settings › Machines is drawn.

mod entry;
mod rows;

pub(crate) use entry::{sidebar_row, title_trigger};

use rows::{child_at, glyph_of, ids, keys_of, subtitle};

use gpui::{FocusHandle, ScrollHandle};
use vessel_app_kit::command::{
    CommandAnimation, CommandFrame, CommandHandlers, CommandKey, CommandRowParts, CommandState,
    CommandText, command_empty, command_frame_animated, command_heading, command_row_parts,
};
use vessel_app_kit::picker;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens};
use vessel_app_view::palette::{
    Direction, PaletteCursor, PaletteEntry, PaletteTarget, marks, palette_entries, palette_groups,
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
    cursor: PaletteCursor,
    /// The list's scroll, to keep the active row in sight.
    scroll: ScrollHandle,
    /// What held the keys before it opened.
    prior: Option<FocusHandle>,
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
            scroll: ScrollHandle::new(),
            prior: None,
        }
    }
}

impl Shell {
    /// Open the palette with an empty search holding the keys; when it is
    /// open already, give its search the keys again.
    pub(crate) fn open_palette(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let placeholder = self.t.lang.palette().placeholder;
        let palette = &mut self.areas.palette;
        if !palette.motion.is_open() {
            palette.prior = window.focused(cx);
            palette.motion.present();
            palette
                .scroll
                .set_offset(vessel_app_kit::ui::point(px(0.0), px(0.0)));
            palette.query.clear();
            palette.cursor.reset();
            palette.search.set_placeholder(placeholder, window, cx);
            palette.search.clear(window, cx);
        }
        palette.search.focus(window, cx);
        cx.notify();
    }

    /// Close the palette; the keys go back where they were.
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let palette = &mut self.areas.palette;
        if !palette.motion.is_open() {
            return;
        }
        palette.motion.close();
        palette.query.clear();
        palette.search.clear(window, cx);
        palette.cursor.reset();
        if let Some(prior) = palette.prior.take() {
            prior.focus(window, cx);
        } else {
            window.blur(cx);
        }
        cx.notify();
    }

    fn palette_searched(&mut self, text: String, cx: &mut Context<'_, Self>) {
        self.areas.palette.query = text;
        self.areas.palette.cursor.reset();
        // A query reveals its highest-ranked row once. Passive fleet
        // refreshes only resolve identity and never commandeer scrolling.
        let all = self.palette_all();
        let palette = &mut self.areas.palette;
        let groups = palette_groups(&all, &palette.query);
        let ids = ids(&groups);
        if let Some(at) = palette
            .cursor
            .resolved(&ids)
            .and_then(|id| child_at(&groups, id))
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
            .map(|section| (section, words.section(section)))
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
            Theme::ALL,
            self.t.lang,
        );
        if !settings::drawn(Section::Overview) {
            all.retain(|entry| !matches!(entry.target, PaletteTarget::Machine(_)));
        }
        all
    }

    fn palette_key(&mut self, key: CommandKey, window: &mut Window, cx: &mut Context<'_, Self>) {
        match key {
            CommandKey::Up | CommandKey::Down => {
                let all = self.palette_all();
                let palette = &mut self.areas.palette;
                let groups = palette_groups(&all, &palette.query);
                let ids = ids(&groups);
                let direction = if key == CommandKey::Down {
                    Direction::Next
                } else {
                    Direction::Previous
                };
                let next = palette.cursor.advance(&ids, direction);
                if let Some(at) = next.and_then(|id| child_at(&groups, id)) {
                    palette.scroll.scroll_to_item(at);
                }
                cx.notify();
            }
            CommandKey::Enter => {
                let all = self.palette_all();
                let palette = &self.areas.palette;
                let groups = palette_groups(&all, &palette.query);
                let ids = ids(&groups);
                let active = palette.cursor.resolved(&ids);
                let target = active
                    .and_then(|id| all.iter().find(|entry| entry.id == id))
                    .map(|entry| entry.target.clone());
                if let Some(target) = target {
                    self.run_palette(target, window, cx);
                }
            }
            CommandKey::Escape => self.close_palette(window, cx),
        }
    }

    /// Close the palette and do what `target` says.
    fn run_palette(&mut self, target: Target, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.close_palette(window, cx);
        match target {
            PaletteTarget::NewSession(machine) => {
                self.go(Route::Draft, window, cx);
                if let Some(machine) = machine {
                    self.choose_machine(machine, window, cx);
                }
            }
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
    let compact = f32::from(window.viewport_size().width) < picker::COMPACT;
    let motion = shell.areas.palette.motion.sample(
        compact,
        window.viewport_size().height,
        cx.reduce_motion(),
    );
    if motion.hidden {
        return None;
    }
    let all = shell.palette_all();
    let words = shell.t.lang.palette();
    let theme = shell.prefs.settings.appearance.theme;
    let query = shell.areas.palette.query.as_str();
    let groups = palette_groups(&all, query);
    let ids = ids(&groups);
    let active = shell.areas.palette.cursor.settle(&ids);
    let palette = &shell.areas.palette;
    let list = if groups.is_empty() {
        command_empty(words.nothing, tokens).into_any_element()
    } else {
        let mut children: Vec<AnyElement> = Vec::new();
        for (at, (group, entries)) in groups.iter().enumerate() {
            children.push(command_heading(words.group(*group), at == 0, tokens).into_any_element());
            for entry in entries {
                let row = CommandRowParts {
                    glyph: Some(glyph_of(&entry.target)),
                    title: CommandText::new(entry.label.clone(), marks(query, &entry.label)),
                    subtitle: entry.detail.as_ref().map(|detail| subtitle(detail, query)),
                    keys: keys_of(&entry.target, shell),
                    checked: matches!(entry.target, PaletteTarget::Theme(shown) if shown == theme),
                    active: active == Some(entry.id.as_str()),
                };
                let target = entry.target.clone();
                let id = SharedString::from(format!("palette-{}", entry.id));
                let arrived = row.active;
                let drawn = command_row_parts(id.clone(), row, compact, tokens).on_click(
                    cx.listener(move |shell, _event, window, cx| {
                        if shell.areas.palette.motion.is_open() {
                            shell.run_palette(target.clone(), window, cx);
                        }
                    }),
                );
                children.push(if arrived {
                    vessel_app_kit::popup::fill_in(
                        SharedString::from(format!("{id}-arrive")),
                        drawn,
                        tokens.neutral.surface1,
                    )
                    .into_any_element()
                } else {
                    drawn.into_any_element()
                });
            }
        }
        div()
            .id("palette-list")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .track_scroll(&palette.scroll)
            .children(children)
            .into_any_element()
    };
    let search = palette.search.bare(tokens, cx);
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
