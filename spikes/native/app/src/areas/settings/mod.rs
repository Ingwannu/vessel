//! Settings, as Paseo's (`screens/settings-screen.tsx`): the sections
//! listed in the sidebar's place under Paseo's two headings, App and the
//! machine (Paseo's Host), with a way back and the machine picker
//! (`nav.rs`); the chosen section's page beside them (`page.rs`). On a
//! phone the list is the page, and a section is pushed over it with a way
//! back (Paseo's compact layout). Which sections are drawn, and so
//! listed, is `sections.rs`.
//!
//! [Decision Log]
//! - Purpose: the page's Settings (`features/settings/*`,
//!   `features/shell/sidebar-settings.tsx`) in the app, with Paseo's look
//!   (the owner's rule: Paseo's app, cloned).
//! - Choice: one route, `Route::Settings`; which section is shown is this
//!   area's state. A section's page is found in `sections::page`, one
//!   line per section: another track's section (Shortcuts is track S3's,
//!   Projects track F's) is one line there and a module of its own, as a
//!   route is one line in `routes.rs`. A section not drawn is not listed.
//! - A new page arrives with Paseo's nothing-to-something as a fade
//!   (`kit::motion::STANDARD`, a surface arriving; the owner's word 39),
//!   instant while motion is held.
//! - Every select and the machine picker share one list (the kit's
//!   picker list); [`List`] says which one is open.
//! - A change goes through [`Shell::prefer`]: the link's store applies the
//!   page's rules (`Prefs::prefer`), the app saves, and what shows at once
//!   (the theme) is applied. `VESSEL_APP_THEME` still pins the scheme for
//!   one run (captures).
//! - A setting is shown once something reads it ([`applied`]); the send
//!   shortcut is read by both composers ([`apply_composer`]).

mod agents;
mod appearance;
mod book;
mod chat;
mod choices;
pub(crate) mod connected;
mod connections;
mod devices;
mod diagnostics;
mod general;
mod general_words;
pub(crate) use general_words::PreparedGeneral;
mod host_agents;
mod language;
mod motion;
mod nav;
mod overview;
pub(crate) mod page;
mod prefer;
mod prepared_refusal;
use prepared_refusal::PreparedRefusal;
mod sections;
mod theme;

pub(crate) use sections::drawn;
mod sync;

use gpui::AnimationExt as _;
use vessel_app_kit::select::{self, Anchor};
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, ElementId, IntoElement as _, ParentElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{ComboHandlers, ComboboxState, SendOn, Tokens, Type, TypeExt as _, combobox};
use vessel_app_view::prefs::{SendKey, send_key_label};
use vessel_app_view::settings::Section;

pub(crate) use choices::{List, primary};
pub(crate) use nav::{section_glyph, sidebar};

use crate::routes::Route;
use crate::shell::Shell;

/// Settings' own state.
pub(crate) struct SettingsState {
    /// Complete storage wording prepared before any frame.
    pub(crate) general: PreparedGeneral,
    /// The section shown.
    pub(crate) section: Section,
    /// On a phone: the section's page is pushed over the list.
    pub(crate) pushed: bool,
    /// How many times a page has arrived (a section chosen, Settings
    /// opened): a new count fades the page in; 0, drawn in place.
    turn: u64,
    /// The machine the per-machine sections are about, by its name in the
    /// book; `None`: the first.
    pub(crate) machine: Option<String>,
    /// Where "Back" goes.
    pub(crate) from: Route,
    /// The one list every select and the machine picker share.
    pub(crate) list: ComboboxState,
    /// Which one it is showing.
    pub(crate) open: Option<List>,
    /// Where each trigger was drawn.
    pub(crate) anchors: Anchors,
    /// The last refused cause and its message prepared before drawing.
    pub(crate) refused: Option<PreparedRefusal>,
    pub(crate) agents: agents::AgentSettings,

    pub(crate) devices: devices::Devices,
    diagnostics: diagnostics::State,
    host_agents: host_agents::State,
    overview: overview::State,
    /// The book as read for the machine pages (Overview, Connections).
    book: book::BookRead,
    language: language::State,
    pub(crate) sync: sync::SyncState,
    /// The Animations switch's changes (its thumb slides on each).
    pub(crate) animations_turns: u64,
}

impl SettingsState {
    pub(crate) fn new(
        general: PreparedGeneral,
        window: &mut Window,
        cx: &mut Context<'_, Shell>,
    ) -> Self {
        Self {
            general,
            section: Section::DEFAULT,
            pushed: false,
            turn: 0,
            machine: None,
            from: Route::Draft,
            list: ComboboxState::new(window, cx),
            open: None,
            anchors: Anchors::default(),
            refused: None,
            agents: agents::AgentSettings::new(window, cx),

            devices: devices::Devices::new(window, cx),
            diagnostics: diagnostics::State::new(cx),
            host_agents: host_agents::State::default(),
            overview: overview::State::new(window, cx),
            book: book::BookRead::default(),
            language: language::State::new(window, cx),
            sync: sync::SyncState::new(),
            animations_turns: 0,
        }
    }

    /// The list showing, if one is.
    pub(crate) fn showing(&self) -> Option<List> {
        self.open.filter(|_| self.list.is_open())
    }

    /// Show `section` (the default when it is not listed now) as a page
    /// arriving.
    fn arrive(&mut self, section: Section, listed: bool, pushed: bool) {
        self.section = if listed { section } else { Section::DEFAULT };
        self.pushed = pushed;
        self.refused = None;
        self.turn = self.turn.saturating_add(1);
    }
}

/// Where each trigger was last drawn.
#[derive(Default)]
pub(crate) struct Anchors {
    machine: Anchor,
    send: Anchor,
    delivery: Anchor,
    interface: Anchor,
    code: Anchor,
    width: Anchor,
    display: Anchor,
}

impl Anchors {
    pub(crate) const fn of(&self, list: List) -> &Anchor {
        match list {
            List::Machine => &self.machine,
            List::Send => &self.send,
            List::Delivery => &self.delivery,
            List::Interface => &self.interface,
            List::Code => &self.code,
            List::Width => &self.width,
            List::Display => &self.display,
        }
    }
}

impl Shell {
    /// Each frame, the Settings pages that wait on the window settle what
    /// came in since (devices, sync, the machine's agents, the book that
    /// Overview and Connections show).
    pub(crate) fn settle_settings(&mut self, window: &Window) {
        self.settle_devices(window);
        self.settle_sync(window);
        self.settle_host_agents(window);
        self.settle_book(window);
    }

    /// Open Settings at `section` (pushed on a phone), or at its start
    /// (General beside the list; the list on a phone, as Paseo's gear),
    /// remembering where "Back" returns to.
    pub(crate) fn open_settings(
        &mut self,
        section: Option<Section>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.route != Route::Settings {
            self.areas.settings.from = self.route;
        }
        // A section not listed now opens the first one instead.
        let section = section.filter(|section| sections::listed(*section, self));
        let state = &mut self.areas.settings;
        state.arrive(section.unwrap_or(Section::DEFAULT), true, section.is_some());
        self.go(Route::Settings, window, cx);
        self.agent_load(false, cx);
    }

    /// Show `section` (from the list).
    pub(crate) fn settings_section(&mut self, section: Section, cx: &mut Context<'_, Self>) {
        let listed = sections::listed(section, self);
        self.areas.settings.arrive(section, listed, true);
        crate::areas::shortcuts::navigation_changed(self);
        self.agent_load(false, cx);
        cx.notify();
    }

    /// Leave Settings, or on a phone go from a section back to the list.
    pub(crate) fn settings_back(
        &mut self,
        to_list: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if to_list {
            self.areas.settings.pushed = false;
            cx.notify();
        } else {
            let from = self.areas.settings.from;
            self.areas.settings.pushed = false;
            self.go(from, window, cx);
        }
    }

    /// Open `list` under its trigger.
    pub(crate) fn open_list(
        &mut self,
        list: List,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.settings.language.close();
        let state = &mut self.areas.settings;
        let drawn = state.anchors.of(list).get();
        let at = match list {
            // Paseo's host picker opens at its trigger's left edge.
            List::Machine => drawn,
            List::Send
            | List::Delivery
            | List::Interface
            | List::Code
            | List::Width
            | List::Display => drawn.map(|bounds| select::end_aligned(bounds, select::WIDTH)),
        };
        state.list.set_anchor(at);
        state.list.open(None, window, cx);
        state.open = Some(list);
        cx.notify();
    }

    /// Row `index` of the open list was chosen.
    fn chose_in_list(&mut self, index: usize, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(list) = self.areas.settings.open.take() else {
            return;
        };
        self.areas.settings.list.close();
        if list == List::Machine {
            match self.fleet.machines.get(index) {
                Some(machine) => self.areas.settings.machine = Some(machine.name.clone()),
                // The row after the machines: adding one.
                None => self.go(Route::Pair, window, cx),
            }
            self.agent_load(false, cx);
            cx.notify();
        } else if let Some(change) = choices::change(list, index) {
            self.prefer(&change, window, cx);
        }
    }
}

/// The theme the preferences name, unless `VESSEL_APP_THEME` pins one for
/// this run.
pub(crate) fn apply_theme(shell: &Shell, window: &Window, cx: &mut App) {
    if std::env::var_os("VESSEL_APP_THEME").is_some() {
        return;
    }
    theme::apply(
        shell.prefs.settings.appearance.theme,
        window.appearance(),
        cx,
    );
}

/// The send shortcut as a person reads it ("Enter", "Ctrl+Enter").
pub(crate) fn send_key_words(shell: &Shell) -> String {
    send_key_label(shell.prefs.settings.composer.send, primary())
}

/// Both composers as the preferences say: which press sends, and the
/// session's placeholder naming it (the new chat's names it when it
/// arrives, `draft::arrive`).
pub(crate) fn apply_composer(shell: &Shell, window: &mut Window, cx: &mut App) {
    let send = match shell.prefs.settings.composer.send {
        SendKey::Enter => SendOn::Enter,
        SendKey::ModEnter => SendOn::ModEnter,
    };
    shell.areas.session.composer.set_send_on(send);
    shell.areas.draft.composer.set_send_on(send);
    let placeholder = shell.t.composer_placeholder(&send_key_words(shell));
    shell
        .areas
        .session
        .composer
        .set_placeholder(&placeholder, window, cx);
}

/// The interface's and code's text sizes, drawn everywhere (the kit's
/// type scale); the conversation's rows measured again at them.
pub(crate) fn apply_text_sizes(shell: &Shell, cx: &mut App) {
    let appearance = &shell.prefs.settings.appearance;
    vessel_app_kit::set_text_sizes(
        cx,
        appearance.interface_size.get(),
        appearance.code_size.get(),
    );
    let list = &shell.areas.conversation.list;
    list.changed(0..list.rows());
    shell.areas.session.entries.remeasure();
}

/// At start: the kept theme, and the system's appearance followed while
/// the theme is "System"; the text sizes; the composers' send shortcut;
/// the conversation's width and display.
pub(crate) fn start(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    devices::start(shell, window, cx);
    apply_theme(shell, window, cx);
    apply_text_sizes(shell, cx);
    apply_composer(shell, window, cx);
    let appearance = shell.prefs.settings.appearance;
    crate::areas::session::set_content_width(appearance.content_width);
    shell.conversation_display(appearance.conversation);
    cx.observe_window_appearance(window, |shell, window, cx| {
        apply_theme(shell, window, cx);
    })
    .detach();
}

/// The Settings screen: on a wide window the chosen section's page (the
/// list is in the sidebar's place); on a phone the list, or the section
/// pushed over it.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let phone = vessel_app_kit::breakpoint::phone(window);
    let state = &shell.areas.settings;
    let body = if phone && !state.pushed {
        nav::compact(shell, tokens, cx).into_any_element()
    } else {
        section_page(shell, tokens, phone, window, cx)
    };
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(body)
        .children(list(shell, tokens, phone, window, cx))
        .children(language::picker(shell, tokens, window, cx))
        .children(crate::areas::about::dialogs(shell, tokens, window, cx))
        .children(agents::dialogs(shell, tokens, window, cx))
        .children(devices::dialogs(shell, tokens, window, cx))
        .children(diagnostics::dialogs(shell, tokens, window, cx))
        .children(overview::dialogs(shell, tokens, window, cx))
}

/// The section shown, as a page: its title, a refused change above its
/// body, the body fading in when it has just arrived.
fn section_page(
    shell: &Shell,
    tokens: &Tokens,
    phone: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = &shell.areas.settings;
    // A section no longer listed (its machine left the book) gives way to
    // the first one, as opening it would.
    let section = if sections::listed(state.section, shell) {
        state.section
    } else {
        Section::DEFAULT
    };
    let title = shell.t.lang.settings().section(section, shell.words);
    let body = sections::draw(section, shell, tokens, phone, window, cx)
        .unwrap_or_else(|| div().into_any_element());
    let body = div()
        .flex()
        .flex_col()
        .children(state.refused.as_ref().map(|refused| {
            div()
                .mb(px(16.0))
                .px(px(4.0))
                .type_(Type::Body)
                .text_color(tokens.neutral.destructive)
                .child(refused.words.clone())
        }))
        .child(body);
    let body = if state.turn == 0 {
        body.into_any_element()
    } else {
        body.with_animation(
            ElementId::NamedInteger("settings-page".into(), state.turn),
            vessel_app_kit::motion::transition(vessel_app_kit::motion::STANDARD),
            gpui::Styled::opacity,
        )
        .into_any_element()
    };
    // Providers (the agents' page) keeps a scroll position per machine and
    // per agent, and an agent's detail has its own header.
    let (scroll_id, detail) = if section == Section::Providers {
        (agents::scroll_id(shell), agents::header(shell))
    } else {
        ("settings-page".into(), None)
    };
    page::layout(title, body, phone, scroll_id, detail, tokens, cx).into_any_element()
}

/// Whether a setting's row is shown: only a setting the app already draws
/// from. These are kept in the preferences but nothing reads them yet
/// (the send key, the default send, the two font sizes, the
/// conversation's width and its display): their rows return when what
/// they change is drawn.
pub(crate) const fn applied(list: List) -> bool {
    match list {
        List::Machine
        | List::Send
        | List::Delivery
        | List::Interface
        | List::Code
        | List::Width
        | List::Display => true,
    }
}

/// The open list, drawn over everything.
fn list(
    shell: &Shell,
    tokens: &Tokens,
    phone: bool,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings;
    let open = state.showing()?;
    let rows = choices::rows(open, shell, tokens);
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.settings.list,
        chose: std::rc::Rc::new(|shell: &mut Shell, index, window, cx| {
            shell.chose_in_list(index, window, cx);
        }),
        back: None,
    };
    let (header, width) = match open {
        List::Machine => (nav::picker_header(shell, phone), nav::PICKER_WIDTH),
        List::Send
        | List::Delivery
        | List::Interface
        | List::Code
        | List::Width
        | List::Display => (select::header(), select::WIDTH),
    };
    combobox(
        "settings-list",
        &state.list,
        &header,
        &rows,
        &handlers,
        width,
        tokens,
        window,
        cx,
    )
}
