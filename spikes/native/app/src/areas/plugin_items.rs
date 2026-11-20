//! A machine's plugin items in the app (ADR 0115, "Catalog and invoke"):
//! the catalog watchers' calls and answers, where an action is taken, and
//! running an item's method. The palette and the composer read the
//! catalogs (`view::plugins::Catalogs`); this is the page's
//! `catalog-sync.tsx` and `invokePlugin` for the window.
//!
//! [Decision Log]
//! - Purpose: Paseo's client keeps every connected host's plugin catalog
//!   (`PluginCatalogSync`) so its Command Center and composer offer the
//!   plugins' items; the app does the same.
//! - Choice: one [`Catalogs`] for the window, followed on each fleet
//!   change (its model decides what to ask); the answers come back as
//!   `Event::PluginCatalog`, apart from the Settings page's, whose tickets
//!   are counted separately.
//! - A method call's failure is a toast in the node's words (Paseo's
//!   `reportError`, the page's toast). Success says nothing: in Paseo the
//!   plugin's own code shows what it wants, and a Vessel plugin's output is
//!   its own until screens (P3b) can draw it.
//! - Where an action is taken: the open session when the session screen
//!   shows it (the page's `/sessions/<machine>/<id>`), else nowhere (a
//!   global item only).

use vessel_app_kit::ui::Context;
use vessel_app_link::Command;
use vessel_app_link::plugins::{Answer, Ask};
use vessel_app_view::board::RunsIn;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::plugins::{
    Call, CatalogAnswered, Place, PluginCall, PluginSlash, problem_words,
};
use vessel_app_view::toasts::ToastTone;

use crate::routes::Route;
use crate::shell::Shell;

impl Shell {
    /// After a fleet change: start or end the machines' catalog watchers.
    pub(crate) fn plugin_catalogs_fleet(&mut self) {
        let calls = self.areas.plugin_catalogs.sync(&self.fleet);
        self.plugin_catalog_calls(calls);
    }

    /// Send the watchers' or a method's calls.
    fn plugin_catalog_calls(&self, calls: Vec<(MachineKey, Call)>) {
        for (machine, Call { ticket, request }) in calls {
            self.command(Command::PluginCatalog(Ask {
                machine,
                ticket,
                request,
            }));
        }
    }

    /// A watcher's call or a method call answered.
    pub(crate) fn plugin_catalog_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        let Answer {
            machine,
            ticket,
            result,
        } = answer;
        match self
            .areas
            .plugin_catalogs
            .answered(&machine, ticket, result)
        {
            CatalogAnswered::Stale | CatalogAnswered::Invoked(Ok(_)) => {}
            CatalogAnswered::Asked { calls, redraw } => {
                self.plugin_catalog_calls(calls);
                if redraw {
                    cx.notify();
                }
            }
            CatalogAnswered::Invoked(Err(problem)) => {
                let text = problem_words(self.words.plugins(), &problem);
                self.toast(text, ToastTone::Error, cx);
            }
        }
    }

    /// Where a plugin action is taken now: the open session while the
    /// session screen shows it, else no session.
    pub(crate) fn plugin_place(&self) -> Place {
        let Some(open) = self
            .areas
            .session
            .open
            .as_ref()
            .filter(|_| self.route == Route::Session)
        else {
            return Place::Global;
        };
        let card = self
            .fleet
            .machines
            .iter()
            .filter(|machine| machine.name == open.machine)
            .flat_map(|machine| &machine.sessions)
            .find(|card| card.session == open.session);
        Place::Session {
            machine: MachineKey::from_book(&open.machine),
            agent: open.session,
            project: card
                .and_then(|card| card.detail.runs_in)
                .map(RunsIn::project),
            worktree: card.and_then(|card| card.detail.worktree),
        }
    }

    /// The open session machine's plugin slash commands, while the session
    /// screen shows it (Paseo's `usePluginClientSlashCommands`).
    pub(crate) fn session_plugin_slash(&self) -> Vec<&PluginSlash> {
        let place = self.plugin_place();
        let Place::Session { machine, .. } = &place else {
            return Vec::new();
        };
        self.areas.plugin_catalogs.slash_commands(machine, &place)
    }

    /// Run a plugin method; its answer comes back as an event.
    pub(crate) fn run_plugin_call(&mut self, call: PluginCall) {
        let asked = self.areas.plugin_catalogs.invoke(call);
        self.plugin_catalog_calls(asked.into_iter().collect());
    }

    /// Run the palette's plugin item `command` as the catalog has it now
    /// (an item gone since the palette drew it does nothing).
    pub(crate) fn run_plugin_command(&mut self, command: &str) {
        let place = self.plugin_place();
        let call = self
            .areas
            .plugin_catalogs
            .commands(&place)
            .into_iter()
            .find(|item| item.id == command)
            .map(|item| item.call);
        if let Some(call) = call {
            self.run_plugin_call(call);
        }
    }
}
