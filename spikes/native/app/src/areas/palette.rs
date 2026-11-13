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

use gpui::{FocusHandle, ScrollHandle};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::breakpoint;
use vessel_app_kit::command::{
    CommandFrame, CommandKey, CommandRow, command_empty, command_frame, command_heading,
    command_row,
};
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens};
use vessel_app_view::palette::{
    PaletteEntry, PaletteGroup, PaletteTarget, kept, mark, moved, palette_entries, palette_groups,
};
use vessel_app_view::prefs::{PreferenceChange, Theme};
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::{Action, Platform, bindings_of, chord_caps};

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
    /// Whether it is on screen.
    open: bool,
    /// Its search.
    search: TextField,
    /// What the search holds.
    query: String,
    /// The row the keyboard is on, by its entry's id; the first listed
    /// when `None` or no longer listed.
    active: Option<String>,
    /// The list's scroll, to keep the active row in sight.
    scroll: ScrollHandle,
    /// What held the keys before it opened.
    prior: Option<FocusHandle>,
}

impl PaletteArea {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: false,
            search: TextField::with_change(
                "",
                |_, _, _, _| {},
                Shell::palette_searched,
                window,
                cx,
            ),
            query: String::new(),
            active: None,
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
        if !palette.open {
            palette.prior = window.focused(cx);
            palette.open = true;
            palette.query.clear();
            palette.active = None;
            palette.search.set_placeholder(placeholder, window, cx);
            palette.search.clear(window, cx);
        }
        palette.search.focus(window, cx);
        cx.notify();
    }

    /// Close the palette; the keys go back where they were.
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let palette = &mut self.areas.palette;
        if !palette.open {
            return;
        }
        palette.open = false;
        if let Some(prior) = palette.prior.take() {
            prior.focus(window, cx);
        }
        cx.notify();
    }

    fn palette_searched(&mut self, text: String, cx: &mut Context<'_, Self>) {
        self.areas.palette.query = text;
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
        let mut all = palette_entries(
            &self.fleet,
            self.areas.draft.machine.as_deref(),
            &sections,
            Theme::ALL,
            self.t.lang,
        );
        if !settings::drawn(Section::Machines) {
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
                let active = kept(palette.active.as_deref(), &ids);
                let next = moved(active, &ids, key == CommandKey::Down);
                if let Some(at) = next.and_then(|id| child_at(&groups, id)) {
                    palette.scroll.scroll_to_item(at);
                }
                palette.active = next.map(str::to_owned);
                cx.notify();
            }
            CommandKey::Enter => {
                let all = self.palette_all();
                let palette = &self.areas.palette;
                let groups = palette_groups(&all, &palette.query);
                let ids = ids(&groups);
                let active = kept(palette.active.as_deref(), &ids);
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
                self.open_settings(Some(Section::Machines), window, cx);
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

/// The listed entries' ids, top to bottom.
fn ids<'a>(groups: &[(PaletteGroup, Vec<&'a Entry>)]) -> Vec<&'a str> {
    groups
        .iter()
        .flat_map(|(_, entries)| entries.iter().map(|entry| entry.id.as_str()))
        .collect()
}

/// Where the entry `id`'s row is among the list's children (each group's
/// heading is one before its rows).
fn child_at(groups: &[(PaletteGroup, Vec<&Entry>)], id: &str) -> Option<usize> {
    let mut at = 0_usize;
    for (_, entries) in groups {
        at = at.saturating_add(1);
        for entry in entries {
            if entry.id == id {
                return Some(at);
            }
            at = at.saturating_add(1);
        }
    }
    None
}

/// An entry's glyph (the page's `iconOf`).
const fn glyph_of(target: &Target) -> Glyph {
    match target {
        PaletteTarget::NewSession(_) => Glyph::Plus,
        PaletteTarget::AddMachine | PaletteTarget::Machine(_) => Glyph::Machine,
        PaletteTarget::Board => Glyph::Board,
        PaletteTarget::Settings(section) => settings::section_glyph(*section),
        PaletteTarget::Session { structured, .. } => {
            if *structured {
                Glyph::Message
            } else {
                Glyph::Terminal
            }
        }
        PaletteTarget::Theme(Theme::System) => Glyph::Monitor,
        PaletteTarget::Theme(Theme::Light) => Glyph::Sun,
        PaletteTarget::Theme(Theme::Dark) => Glyph::Moon,
    }
}

/// The shortcut that does what `target` does, if one does.
const fn action_of(target: &Target) -> Option<Action> {
    match target {
        PaletteTarget::NewSession(_) => Some(Action::NewSession),
        PaletteTarget::Board => Some(Action::GoBoard),
        PaletteTarget::Settings(section) => match section {
            Section::General => Some(Action::Settings),
            Section::Machines => Some(Action::GoMachines),
            Section::Sync => Some(Action::GoSync),
            Section::Devices => Some(Action::GoDevices),
            Section::Appearance
            | Section::Shortcuts
            | Section::Notifications
            | Section::Agents
            | Section::Models
            | Section::Projects
            | Section::About => None,
        },
        PaletteTarget::AddMachine
        | PaletteTarget::Machine(_)
        | PaletteTarget::Session { .. }
        | PaletteTarget::Theme(_) => None,
    }
}

/// The badges of the first key bound to what `target` does, one per
/// chord (Paseo's `Shortcut`), as the person has the keys.
fn keys_of(target: &Target, shell: &Shell) -> Vec<SharedString> {
    let Some(action) = action_of(target) else {
        return Vec::new();
    };
    let platform = Platform::native();
    let joiner = match platform {
        Platform::Mac => "",
        Platform::Other => "+",
    };
    bindings_of(action, &shell.prefs.settings.shortcuts)
        .first()
        .map(|binding| {
            binding
                .chords()
                .map(|chord| chord_caps(chord, platform).join(joiner).into())
                .collect()
        })
        .unwrap_or_default()
}

/// The palette, while open: over everything, at the window's top (a sheet
/// from the bottom at a phone's width).
pub(crate) fn render(
    shell: &mut Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !shell.areas.palette.open {
        return None;
    }
    let all = shell.palette_all();
    let compact = breakpoint::phone(window);
    let words = shell.t.lang.palette();
    let theme = shell.prefs.settings.appearance.theme;
    let palette = &shell.areas.palette;
    let query = palette.query.as_str();
    let groups = palette_groups(&all, query);
    let ids = ids(&groups);
    let active = kept(palette.active.as_deref(), &ids);
    let list = if groups.is_empty() {
        command_empty(words.nothing, tokens).into_any_element()
    } else {
        let mut children: Vec<AnyElement> = Vec::new();
        for (at, (group, entries)) in groups.iter().enumerate() {
            children.push(command_heading(words.group(*group), at == 0, tokens).into_any_element());
            for entry in entries {
                let row = CommandRow {
                    glyph: Some(glyph_of(&entry.target)),
                    title: entry.label.clone().into(),
                    title_mark: mark(query, &entry.label),
                    subtitle: entry.detail.clone().map(Into::into),
                    subtitle_mark: entry
                        .detail
                        .as_deref()
                        .and_then(|detail| mark(query, detail)),
                    keys: keys_of(&entry.target, shell),
                    checked: matches!(entry.target, PaletteTarget::Theme(shown) if shown == theme),
                    active: active == Some(entry.id.as_str()),
                };
                let target = entry.target.clone();
                let id = SharedString::from(format!("palette-{}", entry.id));
                children.push(
                    command_row(id, row, compact, tokens)
                        .on_click(cx.listener(move |shell, _event, window, cx| {
                            shell.run_palette(target.clone(), window, cx);
                        }))
                        .into_any_element(),
                );
            }
        }
        div()
            .id("palette-list")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .track_scroll(&palette.scroll)
            .pb(px(8.0))
            .children(children)
            .into_any_element()
    };
    let search = palette.search.bare(tokens, cx);
    Some(command_frame(
        CommandFrame {
            search,
            list,
            compact,
        },
        Shell::palette_key,
        Shell::close_palette,
        tokens,
        window,
        cx,
    ))
}
