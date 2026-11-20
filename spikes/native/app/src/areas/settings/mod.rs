//! Settings, as Paseo's (`screens/settings-screen.tsx`): the sections
//! listed in the sidebar's place under the page's headings, with a way
//! back and the machine picker (`nav.rs`); the chosen section's page
//! beside them (`page.rs`). On a phone the list is the page, and a
//! section is pushed over it with a way back (Paseo's compact layout).
//!
//! [Decision Log]
//! - Purpose: the page's Settings (`features/settings/*`,
//!   `features/shell/sidebar-settings.tsx`) in the app, with Paseo's look
//!   (the owner's rule: Paseo's app, cloned).
//! - Choice: one route, `Route::Settings`; which section is shown is this
//!   area's state. A section's page is found in [`content`], one line per
//!   section: another track's section (Shortcuts and Notifications are
//!   track S's, Projects track F's) is one line there and a module of its
//!   own, as a route is one line in `routes.rs`. A section not drawn yet
//!   says so on its page.
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
mod choices;
mod devices;
mod general;
mod language;
mod nav;
pub(crate) mod page;
mod sync;

use vessel_app_kit::select::{self, Anchor};
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, IntoElement as _, ParentElement as _, Styled as _, Window, div,
    px,
};
use vessel_app_kit::{ComboHandlers, ComboboxState, SendOn, Tokens, Type, TypeExt as _, combobox};
use vessel_app_link::Command;
use vessel_app_view::prefs::{PreferenceChange, SendKey, send_key_label};
use vessel_app_view::settings::Section;

pub(crate) use choices::{List, primary};
pub(crate) use nav::sidebar;

use crate::routes::Route;
use crate::shell::Shell;

/// Settings' own state.
pub(crate) struct SettingsState {
    /// The section shown.
    pub(crate) section: Section,
    /// On a phone: the section's page is pushed over the list.
    pub(crate) pushed: bool,
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
    /// The last change the store refused, in words.
    pub(crate) refused: Option<String>,
    pub(crate) agents: agents::AgentSettings,

    pub(crate) devices: devices::Devices,
    language: language::State,
    pub(crate) sync: sync::SyncState,
}

impl SettingsState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            section: Section::DEFAULT,
            pushed: false,
            machine: None,
            from: Route::Draft,
            list: ComboboxState::new(window, cx),
            open: None,
            anchors: Anchors::default(),
            refused: None,
            agents: agents::AgentSettings::new(window, cx),

            devices: devices::Devices::new(window, cx),
            language: language::State::new(window, cx),
            sync: sync::SyncState::new(),
        }
    }

    /// The list showing, if one is.
    pub(crate) fn showing(&self) -> Option<List> {
        self.open.filter(|_| self.list.is_open())
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
        let state = &mut self.areas.settings;
        // A section not drawn yet opens the first one instead.
        let section = section.filter(|section| drawn(*section));
        state.section = section.unwrap_or(Section::DEFAULT);
        state.pushed = section.is_some();
        state.refused = None;
        self.go(Route::Settings, window, cx);
        self.agent_load(false, cx);
    }

    /// Show `section` (from the list).
    pub(crate) fn settings_section(&mut self, section: Section, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings;
        state.section = if drawn(section) {
            section
        } else {
            Section::DEFAULT
        };
        state.pushed = true;
        state.refused = None;
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

    /// Change one preference as the page's rules allow, keep it, and show
    /// what changes at once.
    pub(crate) fn prefer(
        &mut self,
        change: &PreferenceChange,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        // Prepare before persisting: a catalog failure leaves the displayed
        // choice and the saved choice in agreement, and never occurs in paint.
        let language = if let PreferenceChange::Language(preference) = change {
            match self.language.prepare(*preference) {
                Ok(words) => Some(words),
                Err(error) => {
                    tracing::error!(%error, "the embedded translations could not change");
                    return;
                }
            }
        } else {
            None
        };
        match self.prefs.prefer(change) {
            Ok(()) => {
                self.areas.settings.refused = None;
                self.command(Command::SavePrefs(self.prefs.clone()));
                match change {
                    PreferenceChange::Language(_) => {
                        if let Some(words) = language {
                            self.apply_language(words, window, cx);
                        }
                    }
                    PreferenceChange::Theme(_) => apply_theme(self, window, cx),
                    PreferenceChange::Send(_) => apply_composer(self, window, cx),
                    PreferenceChange::InterfaceSize(_)
                    | PreferenceChange::CodeSize(_)
                    | PreferenceChange::ContentSize(_)
                    | PreferenceChange::InterfaceFamily(_)
                    | PreferenceChange::CodeFamily(_) => apply_text_sizes(self, cx),
                    PreferenceChange::ContentWidth(width) => {
                        crate::areas::session::set_content_width(*width);
                        let list = &self.areas.conversation.list;
                        list.changed(0..list.rows());
                    }
                    PreferenceChange::Conversation(display) => {
                        self.conversation_display(*display);
                    }
                    // Read where they are drawn, on the next frame.
                    PreferenceChange::SidebarPanel(_)
                    | PreferenceChange::InspectorPanel(_)
                    | PreferenceChange::Delivery(_)
                    | PreferenceChange::Alerts(_)
                    | PreferenceChange::TitleCount(_)
                    | PreferenceChange::SidebarGroup(_)
                    | PreferenceChange::SidebarSort(_)
                    | PreferenceChange::SidebarEnded(_)
                    | PreferenceChange::Shortcuts { .. }
                    | PreferenceChange::ResetShortcuts
                    | PreferenceChange::DefaultAgent { .. }
                    | PreferenceChange::AgentDefaults { .. }
                    | PreferenceChange::ForgetDefaults { .. } => {}
                }
            }
            Err(refused) => {
                let words = self.t.lang.settings();
                self.areas.settings.refused = Some(format!("{}: {refused}", words.refused));
            }
        }
        cx.notify();
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
    let id = appearance::palette_id(shell.prefs.settings.appearance.theme, window.appearance());
    let next = vessel_app_kit::Tokens::of_theme(id);
    if vessel_app_kit::tokens(cx) != next {
        vessel_app_kit::set_theme(cx, id);
    }
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
    appearance::apply_type(&shell.prefs.settings.appearance, cx);
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
    let appearance = shell.prefs.settings.appearance.clone();
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
        let section = state.section;
        let words = shell.t.lang.settings();
        let title = words.section(section);
        let body = content(section, shell, tokens, phone, window, cx)
            .unwrap_or_else(|| page::elsewhere(words.elsewhere, tokens));
        let body = div()
            .flex()
            .flex_col()
            .children(state.refused.as_ref().map(|refused| {
                div()
                    .mb(px(16.0))
                    .px(px(4.0))
                    .type_(Type::Body)
                    .text_color(tokens.neutral.destructive)
                    .child(refused.clone())
            }))
            .child(body)
            .into_any_element();
        let scroll_id = if section == Section::Agents {
            agents::scroll_id(shell)
        } else {
            "settings-page".into()
        };
        let detail = if section == Section::Agents {
            agents::header(shell)
        } else {
            None
        };
        page::layout(title, body, phone, scroll_id, detail, tokens, cx).into_any_element()
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
}

/// Whether `section` is drawn in this build: Settings lists only these.
/// A section no screen draws yet is not shown at all, not as a placeholder
/// naming the track that will draw it (the coordinator's rule for
/// 0.0.0.0.7: nothing shown may do nothing); it is listed again when its
/// page exists. Keep in step with [`content`].
pub(crate) const fn drawn(section: Section) -> bool {
    matches!(
        section,
        Section::General
            | Section::Appearance
            | Section::About
            | Section::Projects
            | Section::Agents
            | Section::Devices
            | Section::Sync
    )
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

/// Which screen draws `section`'s page: one line each. `None`: not drawn
/// yet.
fn content(
    section: Section,
    shell: &Shell,
    tokens: &Tokens,
    phone: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    match section {
        Section::Agents => Some(agents::render(shell, tokens, phone, window, cx)),
        Section::General => Some(general::render(shell, tokens, phone, cx)),
        Section::Devices => Some(devices::render(shell, tokens, phone, cx)),
        Section::Sync => Some(sync::render(shell, tokens, phone, window, cx)),
        Section::Appearance => Some(appearance::render(shell, tokens, phone, cx)),
        Section::About => Some(crate::areas::about::content(shell, tokens, phone, cx)),
        Section::Projects => Some(crate::areas::workspace::projects_content(
            shell, tokens, phone, cx,
        )),
        // Shortcuts and Notifications are track S's; the per-machine
        // sections are this track's next slice.
        Section::Shortcuts | Section::Notifications | Section::Models | Section::Machines => None,
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
