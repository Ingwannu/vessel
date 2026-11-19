//! One machine's plugins as Settings › Plugins holds them (the page's
//! `use-plugins.ts`): the list, the switch, the change in flight, what the
//! last change said, the removal being asked about, the logs shown and the
//! watch that lists again when something changes.
//!
//! [Decision Log]
//! - Purpose: which answer belongs to which request, and what is asked
//!   next, decided once and tested without a window.
//! - Choice: every request carries a ticket from the screen's one
//!   [`Tickets`] (kept across machines and connections, so an answer meant
//!   for a page that was left never matches a new one), and the page keeps
//!   the ticket of each thing it waits for. An answer whose ticket it does
//!   not wait for changes nothing.
//! - Paseo's order after a change, as the page's: the note, then the list
//!   and the switch read again (`load()` in `finish`). One change at a time:
//!   the controls wait while one is made (`mutation.isPending`).
//! - Removal asks first (Paseo's `confirmDialog` inside its `remove`
//!   action): [`Page::act`] with Remove opens the question; Remove in it
//!   sends; Cancel says "Kept {id}".
//! - Cost: a list asked again while one is read replaces the older ask; its
//!   answer is dropped.

use vessel_proto::plugins::{PluginGeneration, PluginId, PluginListItem};
use vessel_proto::settings::NodeSettings;

use super::{
    Action, Call, Change, Feedback, Listed, Logs, PageState, Pending, Problem, Reached, Reply,
    Request, install_source, page_state,
};
use crate::devices::{MachineKey, Ticket, Tickets};

/// What the first watch asks after: a generation the node cannot have (it
/// counts from zero each start), so it answers its current one at once.
const UNSEEN: PluginGeneration = PluginGeneration::new(u64::MAX);

/// One machine's plugins, while Settings › Plugins shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    machine: MachineKey,
    /// The last list read; `None` until the first answer.
    list: Option<Result<Vec<PluginListItem>, Problem>>,
    listing: Option<Ticket>,
    /// The switch in force; `None` until read.
    enabled: Option<bool>,
    reading: Option<Ticket>,
    change: Option<(Ticket, Pending)>,
    feedback: Option<Feedback>,
    watching: Option<Ticket>,
    seen: Option<PluginGeneration>,
    removing: Option<PluginId>,
    logs: Option<Logs>,
}

/// What an answer asks of the screen.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[must_use]
pub struct Answered {
    /// Whether anything shown may have changed (a repeated watch changes
    /// nothing, so no frame is drawn for it).
    pub changed: bool,
    /// The calls to send next.
    pub calls: Vec<Call>,
    /// An install finished: the field empties (Paseo's
    /// `installForm.reset()`).
    pub installed: bool,
}

/// How one row's controls look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowLook {
    /// A change is being made: the switch and the menu wait.
    pub busy: bool,
    /// This row's own action in flight (its menu row says "Reloading…" or
    /// "Removing…").
    pub mine: Option<Action>,
}

impl Page {
    /// The page for `machine` and its first calls: the list, the switch
    /// and the watch (Paseo's query, its daemon config and its feed).
    pub fn open(machine: MachineKey, tickets: &mut Tickets) -> (Self, Vec<Call>) {
        let mut page = Self {
            machine,
            list: None,
            listing: None,
            enabled: None,
            reading: None,
            change: None,
            feedback: None,
            watching: None,
            seen: None,
            removing: None,
            logs: None,
        };
        let calls = [
            page.list_again(tickets),
            page.read_switch(tickets),
            page.watch(tickets, UNSEEN),
        ]
        .into_iter()
        .flatten()
        .collect();
        (page, calls)
    }

    /// The machine it is about.
    #[must_use]
    pub const fn machine(&self) -> &MachineKey {
        &self.machine
    }

    /// What it shows (the machine is connected and speaks the calls, or
    /// the page would not be open).
    #[must_use]
    pub fn state(&self) -> PageState {
        page_state(
            Reached {
                connected: true,
                supported: true,
            },
            Listed {
                loading: self.list.is_none(),
                error: self.list_problem().is_some(),
                count: self.plugins().len(),
            },
        )
    }

    /// The installations of the last list read.
    #[must_use]
    pub fn plugins(&self) -> &[PluginListItem] {
        self.list
            .as_ref()
            .and_then(|list| list.as_ref().ok())
            .map_or(&[], Vec::as_slice)
    }

    /// Why the last list read failed.
    #[must_use]
    pub fn list_problem(&self) -> Option<&Problem> {
        self.list.as_ref().and_then(|list| list.as_ref().err())
    }

    /// Whether plugins are on for the machine; `None` until read.
    #[must_use]
    pub const fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    /// The change in flight.
    #[must_use]
    pub fn pending(&self) -> Option<&Pending> {
        self.change.as_ref().map(|(_, pending)| pending)
    }

    /// Whether a change is in flight (every control waits).
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.change.is_some()
    }

    /// What the last change said.
    #[must_use]
    pub const fn feedback(&self) -> Option<&Feedback> {
        self.feedback.as_ref()
    }

    /// The installation whose removal is being asked about.
    #[must_use]
    pub const fn removing(&self) -> Option<&PluginId> {
        self.removing.as_ref()
    }

    /// The logs dialog, while open.
    #[must_use]
    pub const fn logs(&self) -> Option<&Logs> {
        self.logs.as_ref()
    }

    /// How `plugin`'s row looks now.
    #[must_use]
    pub fn row(&self, plugin: &PluginId) -> RowLook {
        let mine = match self.pending() {
            Some(Pending::Row(action, of)) if of == plugin => Some(*action),
            Some(Pending::Install | Pending::Switch(_) | Pending::Row(..)) | None => None,
        };
        RowLook {
            busy: self.busy(),
            mine,
        }
    }

    /// Retry a failed list (Paseo's `refetch`).
    pub fn retry(&mut self, tickets: &mut Tickets) -> Option<Call> {
        self.list_again(tickets)
    }

    /// The switch was pressed: plugins `on` or off for the machine; nothing
    /// before it was read or while a change waits (the page's `disabled`).
    pub fn toggle(&mut self, tickets: &mut Tickets, on: bool) -> Option<Call> {
        self.enabled?;
        self.begin(tickets, Pending::Switch(on), Request::Switch(on))
    }

    /// Install what the field holds; nothing while it is blank or a change
    /// waits (Paseo's `canSubmit`).
    pub fn install(&mut self, tickets: &mut Tickets, text: &str) -> Option<Call> {
        let source = install_source(text)?.to_owned();
        self.begin(tickets, Pending::Install, Request::Install(source))
    }

    /// A row's `action` on `plugin` (one that is listed). Reload only while
    /// it is enabled (Paseo's menu); Remove first asks ([`Self::remove`]).
    pub fn act(
        &mut self,
        tickets: &mut Tickets,
        action: Action,
        plugin: &PluginId,
    ) -> Option<Call> {
        let item = self.plugins().iter().find(|item| &item.id == plugin)?;
        if self.busy() || (action == Action::Reload && !item.enabled) {
            return None;
        }
        if action == Action::Remove {
            self.removing = Some(plugin.clone());
            return None;
        }
        self.begin(
            tickets,
            Pending::Row(action, plugin.clone()),
            Request::Act(action, plugin.clone()),
        )
    }

    /// The removal asked about was confirmed: send it.
    pub fn remove(&mut self, tickets: &mut Tickets) -> Option<Call> {
        let plugin = self.removing.take()?;
        self.begin(
            tickets,
            Pending::Row(Action::Remove, plugin.clone()),
            Request::Act(Action::Remove, plugin),
        )
    }

    /// The removal asked about was cancelled: "Kept {id}".
    pub fn keep(&mut self) {
        if let Some(plugin) = self.removing.take() {
            self.feedback = Some(Feedback::Done(Change::Kept(plugin)));
        }
    }

    /// Open `plugin`'s logs, asking for them.
    pub fn open_logs(&mut self, tickets: &mut Tickets, plugin: &PluginId) -> Option<Call> {
        let call = ask(tickets, Request::Logs(plugin.clone()))?;
        self.logs = Some(Logs::asked(plugin.clone(), call.ticket));
        Some(call)
    }

    /// Refresh the logs shown; nothing while they are being read.
    pub fn refresh_logs(&mut self, tickets: &mut Tickets) -> Option<Call> {
        let logs = self.logs.as_mut()?;
        if logs.refreshing() {
            return None;
        }
        let call = ask(tickets, Request::Logs(logs.plugin().clone()))?;
        logs.ask(call.ticket);
        Some(call)
    }

    /// Close the logs.
    pub fn close_logs(&mut self) {
        self.logs = None;
    }

    /// `machine` answered the request `ticket`.
    pub fn answer(
        &mut self,
        tickets: &mut Tickets,
        machine: &MachineKey,
        ticket: Ticket,
        result: Result<Reply, Problem>,
    ) -> Answered {
        if machine != &self.machine {
            return Answered::default();
        }
        if self.listing == Some(ticket) {
            self.listing = None;
            self.list = Some(result.and_then(Reply::into_list));
            return shown();
        }
        if self.reading == Some(ticket) {
            self.reading = None;
            if let Ok(settings) = result.and_then(Reply::into_settings) {
                self.enabled = Some(settings.plugins_enabled);
            }
            return shown();
        }
        if self.watching == Some(ticket) {
            return self.watched(tickets, result);
        }
        if self
            .change
            .as_ref()
            .is_some_and(|(asked, _)| *asked == ticket)
        {
            return self.changed(tickets, result);
        }
        let Some(logs) = self.logs.as_mut() else {
            return Answered::default();
        };
        if logs.answered(ticket, result) {
            shown()
        } else {
            Answered::default()
        }
    }

    fn begin(&mut self, tickets: &mut Tickets, pending: Pending, request: Request) -> Option<Call> {
        if self.busy() {
            return None;
        }
        let call = ask(tickets, request)?;
        self.feedback = None;
        self.change = Some((call.ticket, pending));
        Some(call)
    }

    /// A change finished: its note, then the list and the switch again.
    fn changed(&mut self, tickets: &mut Tickets, result: Result<Reply, Problem>) -> Answered {
        let Some((_, pending)) = self.change.take() else {
            return Answered::default();
        };
        let mut installed = false;
        let done = match pending {
            Pending::Install => result.and_then(Reply::into_item).map(|item| {
                installed = true;
                Change::Installed(item.id)
            }),
            Pending::Switch(on) => result.and_then(Reply::into_settings).map(|settings| {
                self.enabled = Some(settings.plugins_enabled);
                Change::Switched(on)
            }),
            Pending::Row(action, plugin) => result
                .and_then(Reply::into_done)
                .map(|()| Change::Acted(action, plugin)),
        };
        self.feedback = Some(done.map_or_else(Feedback::Failed, Feedback::Done));
        let calls = [self.list_again(tickets), self.read_switch(tickets)]
            .into_iter()
            .flatten()
            .collect();
        Answered {
            changed: true,
            calls,
            installed,
        }
    }

    /// The watch answered: a new generation lists again; any answer asks
    /// again after it; a failure ends the watch (the next connection opens
    /// the page again).
    fn watched(&mut self, tickets: &mut Tickets, result: Result<Reply, Problem>) -> Answered {
        self.watching = None;
        let Ok(generation) = result.and_then(Reply::into_generation) else {
            return Answered::default();
        };
        let moved = self.seen.is_some_and(|seen| seen != generation);
        self.seen = Some(generation);
        let mut calls = Vec::new();
        if moved {
            calls.extend(self.list_again(tickets));
            calls.extend(self.read_switch(tickets));
        }
        calls.extend(self.watch(tickets, generation));
        Answered {
            changed: moved,
            calls,
            installed: false,
        }
    }

    fn list_again(&mut self, tickets: &mut Tickets) -> Option<Call> {
        let call = ask(tickets, Request::List)?;
        self.listing = Some(call.ticket);
        Some(call)
    }

    fn read_switch(&mut self, tickets: &mut Tickets) -> Option<Call> {
        let call = ask(tickets, Request::ReadSwitch)?;
        self.reading = Some(call.ticket);
        Some(call)
    }

    fn watch(&mut self, tickets: &mut Tickets, after: PluginGeneration) -> Option<Call> {
        let call = ask(tickets, Request::Watch(after))?;
        self.watching = Some(call.ticket);
        Some(call)
    }
}

/// A call with the next ticket; none once every ticket was used.
fn ask(tickets: &mut Tickets, request: Request) -> Option<Call> {
    tickets
        .next_ticket()
        .ok()
        .map(|ticket| Call { ticket, request })
}

/// An answer that changed what is shown, asking nothing more.
fn shown() -> Answered {
    Answered {
        changed: true,
        calls: Vec::new(),
        installed: false,
    }
}

/// A reply of another kind than the request's: the link's fault, said as
/// the node's internal refusal.
const fn unexpected() -> Problem {
    Problem::Refused {
        code: vessel_proto::ErrorCode::Internal,
        message: None,
    }
}

impl Reply {
    fn into_list(self) -> Result<Vec<PluginListItem>, Problem> {
        if let Self::List(list) = self {
            Ok(list)
        } else {
            Err(unexpected())
        }
    }

    fn into_settings(self) -> Result<NodeSettings, Problem> {
        if let Self::Settings(settings) = self {
            Ok(settings)
        } else {
            Err(unexpected())
        }
    }

    fn into_item(self) -> Result<PluginListItem, Problem> {
        if let Self::Item(item) = self {
            Ok(item)
        } else {
            Err(unexpected())
        }
    }

    fn into_done(self) -> Result<(), Problem> {
        if self == Self::Done {
            Ok(())
        } else {
            Err(unexpected())
        }
    }

    fn into_generation(self) -> Result<PluginGeneration, Problem> {
        if let Self::Generation(generation) = self {
            Ok(generation)
        } else {
            Err(unexpected())
        }
    }
}
