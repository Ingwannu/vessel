//! Settings › Plugins for the chosen machine (ADR 0115), as Paseo's
//! `HostPluginsPage` (`screens/settings/plugins-page.tsx`): the trust
//! notice, "Enable plugins", the install card and its note, the
//! installations with their switch and "…" menu (Logs, Reload, Remove),
//! the remove question and the logs dialog; Paseo's offline and too-old
//! notices otherwise. The decisions are `view::plugins`; the calls
//! `link::plugins`; the drawing `plugins/{body,rows,dialogs}.rs`.
//!
//! [Decision Log]
//! - Purpose: make Paseo's plugin management reachable in the app, as the
//!   page made it on the web (`features/settings/plugins-section.tsx`).
//! - Existing assets: Settings' per-machine pattern (Agents' node-settings
//!   switch: read when shown, dropped when left; Diagnostics' node log
//!   dialog; Devices' remove question), the kit's alert, badge, switch,
//!   field, button, menu with a phone sheet, and dialog.
//! - Choice: the page opens for a machine and its connection when the
//!   section is shown, and is dropped when it is left, the machine goes
//!   away or the connection changes, so coming back reads again (Paseo's
//!   query and feed live as long as its screen). One [`Tickets`] for every
//!   page, so an answer meant for a page that was left matches nothing.
//! - The field's emptying and its waiting need the window: they are done
//!   before the next frame ([`Shell::settle_plugins`]), as arriving at a
//!   route is.
//! - A switch slides only when the value it shows changes (an answer),
//!   not when pressed: Paseo's waits for its mutation too.
//! - Stated differences: the menu's trigger is the kit's provider-row
//!   trigger (a 14 dots glyph in a 32 square; Paseo's is 18 with 4 around);
//!   the source field keeps its text when another machine is chosen (the
//!   page's resets). The section's glyph in the list is the kit's own
//!   `Blocks` drawing after Paseo's Lucide `Blocks`.
//! - The trigger opens on a pointer's press and on a click (assistive
//!   technology's Click action), once.

mod body;
mod dialogs;
mod rows;

use std::cell::RefCell;
use std::collections::HashMap;

use vessel_app_kit::menu::{MenuState, TriggerAnchor};
use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_kit::{DialogState, Editability, TextField, Tokens};
use vessel_app_link::Command;
use vessel_app_link::plugins::{Answer, Ask};
use vessel_app_view::Reach;
use vessel_app_view::connection::ConnectionEpoch;
use vessel_app_view::devices::{MachineKey, Tickets};
use vessel_app_view::plugins::{Action, Call, Page, speaks};
use vessel_app_view::settings::Section;
use vessel_proto::plugins::PluginId;

use crate::routes::Route;
use crate::shell::Shell;

/// The page open, and the connection it was opened over.
struct Open {
    page: Page,
    epoch: Option<ConnectionEpoch>,
}

/// Settings › Plugins' own state.
pub(crate) struct State {
    open: Option<Open>,
    tickets: Tickets,
    /// The install field (Paseo's "Plugin source").
    field: TextField,
    /// The field waits while a change is made.
    held: bool,
    /// An install finished: the field empties before the next frame.
    clear: bool,
    menu: MenuState,
    /// Whose row the menu is for.
    menu_for: Option<PluginId>,
    /// Where each row's "…" was drawn.
    anchors: RefCell<HashMap<PluginId, TriggerAnchor>>,
    remove_dialog: DialogState,
    logs_dialog: DialogState,
    /// The machine switch's changes and each row's, for their slides.
    turns: u64,
    row_turns: HashMap<PluginId, u64>,
}

impl State {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        let field = TextField::with_change(
            "",
            |shell: &mut Shell, _text, _window, cx| shell.plugins_install(cx),
            |_shell: &mut Shell, _text, cx| cx.notify(),
            window,
            cx,
        );
        Self {
            open: None,
            tickets: Tickets::default(),
            field,
            held: false,
            clear: false,
            menu: MenuState::new(cx),
            menu_for: None,
            anchors: RefCell::default(),
            remove_dialog: DialogState::new(cx),
            logs_dialog: DialogState::new(cx),
            turns: 0,
            row_turns: HashMap::new(),
        }
    }

    fn page(&self) -> Option<&Page> {
        self.open.as_ref().map(|open| &open.page)
    }

    fn page_mut(&mut self) -> Option<&mut Page> {
        self.open.as_mut().map(|open| &mut open.page)
    }

    /// `plugin`'s trigger place, kept across frames.
    fn anchor(&self, plugin: &PluginId) -> TriggerAnchor {
        std::rc::Rc::clone(self.anchors.borrow_mut().entry(plugin.clone()).or_default())
    }
}

impl Shell {
    /// Before drawing: open the page for the chosen machine when the
    /// section shows a connected machine that has the calls; drop it
    /// otherwise. Then what waits on the window: the field's emptying, its
    /// waiting, and its placeholder in the person's language.
    pub(crate) fn settle_plugins(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let shown = self.route == Route::Settings
            && self.areas.settings.section == Section::Plugins
            && (!vessel_app_kit::breakpoint::phone(window) || self.areas.settings.pushed);
        let target = super::choices::chosen_machine(self)
            .filter(|machine| shown && machine.reach == Reach::Online && speaks(machine.protocol))
            .map(|machine| {
                (
                    MachineKey::from_book(&machine.name),
                    machine.connection_epoch,
                )
            });
        let state = &mut self.areas.settings.plugins;
        match target {
            None => {
                state.open = None;
                state.menu.close();
                state.menu_for = None;
            }
            Some((machine, epoch))
                if !state
                    .open
                    .as_ref()
                    .is_some_and(|open| open.page.machine() == &machine && open.epoch == epoch) =>
            {
                let (page, calls) = Page::open(machine, &mut state.tickets);
                state.open = Some(Open { page, epoch });
                state.menu.close();
                state.menu_for = None;
                state.anchors.borrow_mut().clear();
                state.row_turns.clear();
                self.refresh_plugin_words(window, cx);
                self.plugin_calls(calls);
            }
            Some(_) => {}
        }
        let state = &mut self.areas.settings.plugins;
        if std::mem::take(&mut state.clear) {
            state.field.clear(window, cx);
        }
        let busy = state.page().is_some_and(Page::busy);
        if state.held != busy {
            state.held = busy;
            let editability = if busy {
                Editability::ReadOnly
            } else {
                Editability::Editable
            };
            state.field.set_editability(editability, cx);
        }
    }

    /// The field's placeholder in the person's language (registered with
    /// the language change's placeholders).
    pub(crate) fn refresh_plugin_words(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let placeholder = self.words.plugins().source_placeholder();
        self.areas
            .settings
            .plugins
            .field
            .set_placeholder(placeholder, window, cx);
    }

    /// Send what the page asked for, to its machine.
    fn plugin_calls(&self, calls: Vec<Call>) {
        let Some(page) = self.areas.settings.plugins.page() else {
            return;
        };
        let machine = page.machine().clone();
        for Call { ticket, request } in calls {
            self.command(Command::Plugins(Ask {
                machine: machine.clone(),
                ticket,
                request,
            }));
        }
    }

    /// Run `step` on the open page with the tickets; send the call it
    /// makes.
    fn plugin_step(
        &mut self,
        step: impl FnOnce(&mut Page, &mut Tickets) -> Option<Call>,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.plugins;
        let Some(open) = state.open.as_mut() else {
            return;
        };
        let call = step(&mut open.page, &mut state.tickets);
        self.plugin_calls(call.into_iter().collect());
        cx.notify();
    }

    /// The machine's switch was pressed.
    fn plugins_toggle(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        self.plugin_step(|page, tickets| page.toggle(tickets, on), cx);
    }

    /// Install what the field holds (its button, or Enter in it).
    fn plugins_install(&mut self, cx: &mut Context<'_, Self>) {
        let text = self.areas.settings.plugins.field.text(cx);
        self.plugin_step(|page, tickets| page.install(tickets, &text), cx);
    }

    /// Read the list again after it failed.
    fn plugins_retry(&mut self, cx: &mut Context<'_, Self>) {
        self.plugin_step(Page::retry, cx);
    }

    /// A row's action; Remove asks first, in its dialog.
    fn plugins_act(
        &mut self,
        action: Action,
        plugin: &PluginId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.plugin_step(|page, tickets| page.act(tickets, action, plugin), cx);
        let state = &self.areas.settings.plugins;
        if action == Action::Remove && state.page().and_then(Page::removing).is_some() {
            state.remove_dialog.focus(window, cx);
        }
    }

    /// The machine answered a plugin call: a frame is drawn only when
    /// something shown changed (a watch that saw no change draws nothing;
    /// the shell's event loop leaves this event's drawing to this).
    pub(crate) fn plugins_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        let Answer {
            machine,
            ticket,
            result,
        } = answer;
        let state = &mut self.areas.settings.plugins;
        let Some(open) = state.open.as_mut() else {
            return;
        };
        let page = &mut open.page;
        let was_on = page.enabled();
        let rows_were: HashMap<PluginId, bool> = page
            .plugins()
            .iter()
            .map(|plugin| (plugin.id.clone(), plugin.enabled))
            .collect();
        let answered = page.answer(&mut state.tickets, &machine, ticket, result);
        if was_on.is_some_and(|was| page.enabled() != Some(was)) {
            state.turns = state.turns.wrapping_add(1);
        }
        for plugin in page.plugins() {
            if rows_were
                .get(&plugin.id)
                .is_some_and(|was| *was != plugin.enabled)
            {
                let turn = state.row_turns.entry(plugin.id.clone()).or_default();
                *turn = turn.wrapping_add(1);
            }
        }
        state.clear |= answered.installed;
        self.plugin_calls(answered.calls);
        if answered.changed {
            cx.notify();
        }
    }
}

/// The Plugins page.
pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    body::render(shell, tokens, compact, cx)
}

/// The remove question or the logs dialog, while one is open.
pub(super) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    dialogs::remove(shell, tokens, window, cx).or_else(|| dialogs::logs(shell, tokens, window, cx))
}
