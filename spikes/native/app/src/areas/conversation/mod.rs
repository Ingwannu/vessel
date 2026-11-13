//! A session's conversation and its frame: the timeline, the title bar
//! with its Session actions, the confirms, and (next) find, the outline,
//! the inspector's Summary and a terminal session's view. The page's
//! `features/session` less the composer and its docks, which are the
//! session area's (the lead's); drawn after GOAT's conversation and
//! Paseo's controls (the owner's rule after build 0.0.0.0.3).
//!
//! What it means is `view::timeline`, `view::session` and
//! `view::terminal`; what it asks of the node is `link::Act`; its words
//! are `lang.conversation()`.
//!
//! [Decision Log]
//! - Purpose: the session screen shows the whole conversation as the
//!   page does (every entry kind, a long log scrolling smoothly) and
//!   offers the page's Session actions.
//! - Constraint: the session area (`session.rs`) is the lead's: it keeps
//!   its own fold for the composer and the permission dock, and its frame.
//! - Choice: this area keeps a `Timeline` of its own, fed from the same
//!   records (the lead's `records` hands them over in one line), and
//!   draws the frame's title bar and body for the lead's `render` to
//!   place. Two folds of one log cost a second pass over each record; in
//!   exchange neither area reaches into the other's state, and the lead's
//!   fold goes once the composer reads this one.
//! - Cost: a record is folded twice until then.

mod bar;
mod bench;
mod dock;
mod entries;
mod find;
mod look;
mod outline;
mod rename;
mod timeline;
mod tool;

use std::collections::HashSet;
use std::rc::Rc;

use vessel_app_kit::ui::{App, Context, Subscription};
use vessel_app_kit::{DialogState, MenuHandlers, MenuState, VirtualList};
use vessel_app_link::{Act, ActAnswer, ActKind, Acted, Command};
use vessel_app_view::Card;
use vessel_app_view::prefs::ConversationDisplay;
use vessel_app_view::session::MarkAction;
use vessel_app_view::timeline::{FindCache, Folds, Logs, OutlineItem, Rows, Timeline, outline};
use vessel_proto::{EventRecord, Mark, SessionId, SessionInfo};

pub(crate) use bar::bar;
pub(crate) use dock::dock;
pub(crate) use timeline::{overlays, timeline};

use crate::shell::Shell;

/// The session this area shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shown {
    pub(crate) machine: String,
    pub(crate) session: SessionId,
}

/// A confirm the area has open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Confirm {
    /// Archive the session (a live one is stopped first).
    Archive,
    /// Delete it for good.
    Delete,
    /// Kill it.
    Kill,
}

/// One row of the Session actions menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAction {
    /// A mark or a life action.
    Mark(MarkAction),
    /// Copy the session's id.
    CopyId,
    /// Kill a live session.
    Kill,
}

/// The conversation area's state.
pub(crate) struct ConversationArea {
    /// The session shown, when one is.
    pub(crate) shown: Option<Shown>,
    /// Its conversation, folded from its records.
    pub(crate) timeline: Timeline,
    /// Runs of tool calls folded into one row ("Collapse tool calls").
    pub(crate) folds: Option<Folds>,
    /// The runs opened, by their lead entry.
    pub(crate) open_runs: HashSet<usize>,
    /// The agent's own log lines, by run (`view::timeline::logs`).
    pub(crate) logs: Logs,
    /// The log runs opened, by their first line's entry.
    pub(crate) open_logs: HashSet<usize>,
    /// Whether tool calls are collapsed (Settings' "Collapse tool calls"
    /// against "Full"); see [`collapsing`].
    collapse: bool,
    /// Entries whose disclosure the person toggled (a tool card, a
    /// thought), against its default.
    pub(crate) toggled: HashSet<usize>,
    /// The rows drawn.
    pub(crate) rows: Rows,
    /// The long list.
    pub(crate) list: VirtualList,
    /// The session as the node last described it (marks, abilities,
    /// title); `None` until read.
    pub(crate) info: Option<Box<SessionInfo>>,
    /// The last act the node refused, and its words: shown beside the
    /// control it is about.
    pub(crate) refused: Option<(ActKind, String)>,
    /// The Session actions menu, and what its rows do.
    pub(crate) menu: MenuState,
    pub(crate) menu_rows: Vec<MenuAction>,
    /// The confirm open, if any.
    pub(crate) confirm: Option<Confirm>,
    pub(crate) dialog: DialogState,
    /// The title, while it is renamed in place.
    pub(crate) rename: Option<rename::Rename>,
    /// The prompts, for the outline rail.
    pub(crate) outline: Vec<OutlineItem>,
    /// Where the pointer is on the rail.
    pub(crate) outline_pointer: outline::Pointer,
    /// The find bar, while it is open.
    pub(crate) find: Option<find::FindBar>,
    /// Each entry's folded text for find, kept between searches of the
    /// session shown.
    pub(crate) find_cache: FindCache,
    /// Hears Mod+F (`find`); made once a session is first shown.
    keys: Option<Subscription>,
    /// The scrolling measurement, when it is on.
    bench: bench::Bench,
}

impl ConversationArea {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            shown: None,
            timeline: Timeline::default(),
            folds: None,
            open_runs: HashSet::new(),
            logs: Logs::default(),
            open_logs: HashSet::new(),
            collapse: collapsing(None),
            toggled: HashSet::new(),
            rows: Rows::All(0),
            list: VirtualList::new(true),
            info: None,
            refused: None,
            menu: MenuState::new(cx),
            menu_rows: Vec::new(),
            confirm: None,
            dialog: DialogState::new(cx),
            rename: None,
            outline: Vec::new(),
            outline_pointer: outline::Pointer::default(),
            find: None,
            find_cache: FindCache::default(),
            keys: None,
            bench: bench::Bench::default(),
        }
    }

    /// The rows again, after the entries, the folds or the open runs
    /// changed.
    fn rows_again(&mut self) {
        self.rows = Rows::of(
            self.timeline.entries().len(),
            self.folds.as_ref(),
            &self.logs,
            &self.open_runs,
        );
    }
}

/// Whether the conversation collapses tool calls: as Settings ›
/// Appearance › Conversation display says (`display`; "Full": every tool
/// call a row, an agent's log as one folded row; "Collapse tool calls":
/// runs of tool calls one row each, the log not shown), unless
/// `VESSEL_APP_CONVERSATION` (`full` or `collapse`) pins it for a run, for
/// captures. Before the preferences are read (`None`), collapsed, as the
/// app has drawn it since its first build.
fn collapsing(display: Option<ConversationDisplay>) -> bool {
    match std::env::var("VESSEL_APP_CONVERSATION").as_deref() {
        Ok("full") => false,
        Ok("collapse") => true,
        _ => display.is_none_or(|display| display == ConversationDisplay::CollapseTools),
    }
}

impl Shell {
    /// The session area opened `session` on `machine`: start again from
    /// its first record, and read what the node says of it.
    pub(crate) fn conversation_opened(
        &mut self,
        machine: &str,
        session: SessionId,
        cx: &mut Context<'_, Self>,
    ) {
        self.hear_find(cx);
        let area = &mut self.areas.conversation;
        area.shown = Some(Shown {
            machine: machine.to_owned(),
            session,
        });
        area.timeline = Timeline::default();
        area.folds = area.collapse.then(Folds::default);
        area.open_runs.clear();
        area.logs = Logs::default();
        area.open_logs.clear();
        area.toggled.clear();
        area.rows = Rows::All(0);
        area.list.reset(0);
        area.info = None;
        area.refused = None;
        area.menu.close();
        area.confirm = None;
        area.rename = None;
        area.find = None;
        area.find_cache = FindCache::default();
        area.outline.clear();
        area.outline_pointer = outline::Pointer::default();
        self.command(Command::Act {
            machine: machine.to_owned(),
            session,
            act: Act::Info,
        });
    }

    /// The conversation as Settings' display says: its runs folded or
    /// not, every row measured again (the list starts over at the end).
    pub(crate) fn conversation_display(&mut self, display: ConversationDisplay) {
        let collapse = collapsing(Some(display));
        let area = &mut self.areas.conversation;
        if area.collapse == collapse {
            return;
        }
        area.collapse = collapse;
        area.folds = collapse.then(|| Folds::of(area.timeline.entries()));
        area.open_runs.clear();
        area.rows_again();
        area.list.reset(area.rows.len());
    }

    /// Records of the session followed.
    pub(crate) fn conversation_records(
        &mut self,
        session: &SessionId,
        records: &[EventRecord],
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.conversation;
        if area
            .shown
            .as_ref()
            .is_none_or(|shown| shown.session != *session)
        {
            return;
        }
        let rows_before = area.rows.len();
        let change = area.timeline.apply(records);
        if change.is_empty(area.timeline.entries().len()) {
            return;
        }
        if area.folds.is_some() {
            area.folds = Some(Folds::of(area.timeline.entries()));
        }
        area.logs = Logs::of(area.timeline.entries());
        area.outline = outline(area.timeline.entries());
        area.rows_again();
        let rows_after = area.rows.len();
        // Rows are entries less folded members, so a batch can add rows
        // only at the end and change rows anywhere; measure again from the
        // first row it touched.
        let first_changed = change
            .changed
            .first()
            .copied()
            .unwrap_or(change.added_from)
            .min(change.added_from);
        let from = area
            .rows
            .row_index(first_changed, area.folds.as_ref(), &area.logs)
            .unwrap_or(rows_before)
            .min(rows_before);
        area.list
            .splice(from..rows_before, rows_after.saturating_sub(from));
        self.find_grew(cx);
    }

    /// A session act answered.
    pub(crate) fn acted(&mut self, answer: ActAnswer, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        match answer.result {
            Ok(Acted::Session(info)) => {
                area.refused = None;
                let card = Card::from_session(&answer.machine, &info);
                if let Some(machine) = self
                    .fleet
                    .machines
                    .iter_mut()
                    .find(|machine| machine.name == answer.machine)
                    && let Some(slot) = machine
                        .sessions
                        .iter_mut()
                        .find(|slot| slot.session == card.session)
                {
                    *slot = card;
                }
                if area
                    .shown
                    .as_ref()
                    .is_some_and(|shown| shown.session == info.id)
                {
                    area.info = Some(info);
                }
            }
            Ok(Acted::Deleted(deleted)) => {
                area.refused = None;
                area.info = None;
                if let Some(machine) = self
                    .fleet
                    .machines
                    .iter_mut()
                    .find(|machine| machine.name == answer.machine)
                {
                    machine
                        .sessions
                        .retain(|card| card.session != answer.session);
                }
                let words = self.t.lang.conversation();
                let kept = deleted
                    .worktree
                    .as_ref()
                    .map(|worktree| words.deleted_worktree(&worktree.outcome));
                self.notice = Some(kept.map_or_else(
                    || words.session_deleted().to_owned(),
                    |kept| format!("{}. {kept}", words.session_deleted()),
                ));
            }
            Ok(Acted::Done) => area.refused = None,
            Err(failed) => area.refused = Some((answer.kind, failed.message)),
        }
        cx.notify();
    }

    /// Ask the node to act on the session shown.
    pub(crate) fn act(&self, act: Act) {
        if let Some(shown) = &self.areas.conversation.shown {
            self.command(Command::Act {
                machine: shown.machine.clone(),
                session: shown.session,
                act,
            });
        }
    }

    /// Open or close a run of tool calls.
    pub(crate) fn toggle_run(&mut self, lead: usize, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        if !area.open_runs.remove(&lead) {
            let _new = area.open_runs.insert(lead);
        }
        let before = area.rows.len();
        area.rows_again();
        let after = area.rows.len();
        // Only the run's own rows change: its lead's row stays where it
        // was and its members come or go right after it. Splicing there
        // (not resetting the list) keeps the view where the person is.
        match area.rows.row_index(lead, area.folds.as_ref(), &area.logs) {
            Some(row) => {
                let next = row.saturating_add(1);
                let gone = before.saturating_sub(after);
                area.list.splice(
                    next..next.saturating_add(gone),
                    after.saturating_sub(before),
                );
                area.list.changed(row..next);
            }
            None => area.list.reset(after),
        }
        cx.notify();
    }

    /// Open or close a run of the agent's log lines: only its own row
    /// changes (the lines show in its box, never as rows).
    pub(crate) fn toggle_log(&mut self, lead: usize, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        if !area.open_logs.remove(&lead) {
            let _new = area.open_logs.insert(lead);
        }
        if let Some(row) = area.rows.row_index(lead, area.folds.as_ref(), &area.logs) {
            area.list.changed(row..row.saturating_add(1));
        }
        cx.notify();
    }

    /// Open or close one entry's disclosure (a tool card, a thought).
    pub(crate) fn toggle_entry(&mut self, entry: usize, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        if !area.toggled.remove(&entry) {
            let _new = area.toggled.insert(entry);
        }
        if let Some(row) = area.rows.row_index(entry, area.folds.as_ref(), &area.logs) {
            area.list.changed(row..row.saturating_add(1));
        }
        cx.notify();
    }

    /// The menu's handlers: how to reach its state, and what a row does.
    pub(crate) fn menu_handlers() -> MenuHandlers<Self> {
        MenuHandlers {
            state: |shell: &mut Self| &mut shell.areas.conversation.menu,
            chose: Rc::new(|shell: &mut Self, index, window, cx| {
                let Some(action) = shell.areas.conversation.menu_rows.get(index).copied() else {
                    return;
                };
                shell.menu_chose(action, window, cx);
            }),
        }
    }

    fn menu_chose(
        &mut self,
        action: MenuAction,
        window: &mut vessel_app_kit::ui::Window,
        cx: &mut Context<'_, Self>,
    ) {
        let confirm = match action {
            MenuAction::Mark(MarkAction::Archive) => Some(Confirm::Archive),
            MenuAction::Mark(MarkAction::Delete) => Some(Confirm::Delete),
            MenuAction::Kill => Some(Confirm::Kill),
            MenuAction::Mark(mark) => {
                self.mark(mark);
                None
            }
            MenuAction::CopyId => {
                if let Some(shown) = &self.areas.conversation.shown {
                    cx.write_to_clipboard(vessel_app_kit::ui::ClipboardItem::new_string(
                        shown.session.to_string(),
                    ));
                    self.notice = Some(self.t.lang.conversation().copied_id().to_owned());
                }
                None
            }
        };
        if let Some(confirm) = confirm {
            self.areas.conversation.confirm = Some(confirm);
            self.areas.conversation.dialog.focus(window, cx);
        }
        cx.notify();
    }

    /// A mark or a life action, without a confirm.
    fn mark(&self, action: MarkAction) {
        let act = match action {
            MarkAction::Pin => Act::Mark(Mark::Pin),
            MarkAction::Unpin => Act::Mark(Mark::Unpin),
            MarkAction::MarkUnread => Act::Mark(Mark::Unread),
            MarkAction::MarkRead => Act::Mark(Mark::Read {
                through: self.areas.conversation.timeline.next(),
            }),
            MarkAction::Archive => Act::Mark(Mark::Archive),
            MarkAction::Unarchive => Act::Mark(Mark::Unarchive),
            MarkAction::StartAgain => Act::StartAgain,
            MarkAction::Delete => Act::Delete,
        };
        self.act(act);
    }

    /// The confirm open was accepted.
    pub(crate) fn confirmed(&mut self, cx: &mut Context<'_, Self>) {
        let Some(confirm) = self.areas.conversation.confirm.take() else {
            return;
        };
        match confirm {
            Confirm::Archive => self.mark(MarkAction::Archive),
            Confirm::Delete => self.mark(MarkAction::Delete),
            Confirm::Kill => self.act(Act::Kill),
        }
        cx.notify();
    }

    /// The confirm open was dismissed.
    pub(crate) fn dismiss_confirm(
        &mut self,
        _window: &mut vessel_app_kit::ui::Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.conversation.confirm = None;
        cx.notify();
    }
}
