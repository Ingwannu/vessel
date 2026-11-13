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
//!
//! [Decision Log] Another area's conversation ([`Seat`]).
//! - Purpose: a side question (track N5) is a conversation beside the
//!   session's, drawn as GOAT's `SideChatSurface` embeds its normal chat
//!   view: the same rows, tool cards, thoughts and toggles, not a copy.
//! - Constraint: every row and toggle read and changed
//!   `shell.areas.conversation`, the session area's one conversation.
//! - Alternatives: a state of its own for the rows only (a second model
//!   beside this one, and every fold and row rule kept twice); the rows
//!   drawn from a borrowed state with closures for their toggles (a
//!   closure per row, per frame).
//! - Choice: the state stays one type, [`ConversationArea`]; another area
//!   keeps one of its own, opens it ([`ConversationArea::open`]), feeds it
//!   its session's records ([`ConversationArea::take`]) and draws it with
//!   [`timeline::rows`]. A [`Seat`] says where the state is: two plain functions
//!   from the shell to it, so the rows' toggles reach the same state
//!   they were drawn from. The session area's is [`Seat::SESSION`].
//! - Cost: an embedded conversation carries the frame's parts it does not
//!   use (the menu, the confirm, find), which stay empty.

mod bar;
mod bench;
/// The slot over the composer, and the requests' cards for other areas
/// ([`dock::requests`]).
pub(crate) mod dock;
mod entries;
mod find;
mod look;
mod outline;
mod permission;
mod question;
mod queue;
mod rename;
/// Its rows, for another area's conversation too ([`timeline::rows`]).
pub(crate) mod timeline;
mod tool;

use std::collections::HashSet;
use std::rc::Rc;

use vessel_app_kit::ui::{App, Context, Subscription};
use vessel_app_kit::{DialogState, MenuHandlers, MenuState, Reveal, VirtualList};
use vessel_app_link::{Act, ActAnswer, ActKind, Acted, Command};
use vessel_app_view::Card;
use vessel_app_view::prefs::ConversationDisplay;
use vessel_app_view::session::MarkAction;
use vessel_app_view::timeline::{
    FindCache, Folds, Live, Logs, OutlineItem, Rows, Timeline, outline,
};
use vessel_proto::{
    EventRecord, EventSeq, Mark, PermissionId, PermissionOptionId, QuestionAnswer, QuestionId,
    SessionId, SessionInfo,
};

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

/// Where a conversation's state is kept, for the rows drawn from it and
/// the toggles they answer: the session area's ([`Seat::SESSION`]), or
/// another area's own [`ConversationArea`] (a side question's panel).
/// `get` and `get_mut` must reach the same state.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Seat {
    /// The state, to draw from; `None` once it is gone (a panel closed).
    pub(crate) get: fn(&Shell) -> Option<&ConversationArea>,
    /// The same state, to change.
    pub(crate) get_mut: fn(&mut Shell) -> Option<&mut ConversationArea>,
}

impl Seat {
    /// The session area's conversation.
    pub(crate) const SESSION: Self = Self {
        get: session_seat,
        get_mut: session_seat_mut,
    };
}

/// An answer the person gave on a request's card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Answer {
    /// A permission request's option.
    Permit(PermissionId, PermissionOptionId),
    /// A question's form, filled in or declined.
    Question(QuestionId, QuestionAnswer),
}

/// A [`Seat`] known by its type, for the parts whose controls take plain
/// functions (a question card's text box calls a function, which cannot
/// carry a seat): each place gets its own copy of those functions. The
/// session area's is [`SessionPlace`]; another area answering a
/// conversation's requests in place (the inbox) names its own, and may
/// send the answers its own way.
pub(crate) trait Place: 'static {
    /// Where its conversation is kept.
    const SEAT: Seat;

    /// Send `answer` for the session the conversation shows. The card has
    /// already marked it as going (`permit_sending`, the question's
    /// `sent`); a failure is said on the card by setting the
    /// conversation's `refused` to the act's kind and its words. By
    /// default, the session's own act.
    fn answer(shell: &mut Shell, answer: Answer, cx: &mut Context<'_, Shell>) {
        let act = match answer {
            Answer::Permit(id, option) => Act::Permit(id, option),
            Answer::Question(id, answer) => Act::Answer(id, answer),
        };
        shell.act_at(Self::SEAT, act);
        cx.notify();
    }

    /// Whether the cards hold every answer back (one of the place's own
    /// is still on its way). By default, never.
    fn busy(_shell: &Shell) -> bool {
        false
    }
}

/// The session area's conversation, as a [`Place`].
pub(crate) struct SessionPlace;

impl Place for SessionPlace {
    const SEAT: Seat = Seat::SESSION;
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a seat's reader may find no state; the session area's always has one"
)]
const fn session_seat(shell: &Shell) -> Option<&ConversationArea> {
    Some(&shell.areas.conversation)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a seat's reader may find no state; the session area's always has one"
)]
const fn session_seat_mut(shell: &mut Shell) -> Option<&mut ConversationArea> {
    Some(&mut shell.areas.conversation)
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
    /// A queued message edited in its row.
    pub(crate) queue_edit: Option<queue::QueueEdit>,
    /// The question waiting first, being answered over the composer.
    pub(crate) answering: Option<question::Answering>,
    /// A permission's answer being sent.
    pub(crate) permit_sending: Option<permission::Sending>,
    /// Whether a plan request's plan is folded (Paseo's plan card opens
    /// unfolded).
    pub(crate) plan_folded: bool,
    /// What the node says waits in the session now, for the requests and
    /// questions whose records the log read no longer holds
    /// (`view::timeline::Live`).
    pub(crate) live: Live,
    /// The record to bring into view once it is read
    /// ([`Self::reveal`]).
    reveal: Option<EventSeq>,
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
            queue_edit: None,
            answering: None,
            permit_sending: None,
            plan_folded: false,
            live: Live::default(),
            reveal: None,
            outline: Vec::new(),
            outline_pointer: outline::Pointer::default(),
            find: None,
            find_cache: FindCache::default(),
            keys: None,
            bench: bench::Bench::default(),
        }
    }

    /// Show `session` on `machine`: what was shown before is dropped, and
    /// the session's records are folded from its first again
    /// ([`Self::take`]). Asks the node nothing.
    pub(crate) fn open(&mut self, machine: &str, session: SessionId) {
        self.shown = Some(Shown {
            machine: machine.to_owned(),
            session,
        });
        self.timeline = Timeline::default();
        self.folds = self.collapse.then(Folds::default);
        self.open_runs.clear();
        self.logs = Logs::default();
        self.open_logs.clear();
        self.toggled.clear();
        self.rows = Rows::All(0);
        self.list.reset(0);
        self.info = None;
        self.refused = None;
        self.menu.close();
        self.confirm = None;
        self.rename = None;
        self.find = None;
        self.find_cache = FindCache::default();
        self.outline.clear();
        self.queue_edit = None;
        self.answering = None;
        self.permit_sending = None;
        self.plan_folded = false;
        self.live = Live::default();
        self.reveal = None;
        self.outline_pointer = outline::Pointer::default();
    }

    /// Bring record `seq` into view (the inbox opening a session at the
    /// request it lists): now if it is read, else once it is; the first
    /// row when the log no longer holds it (its request is then drawn from
    /// the node's word, [`Self::live`]).
    #[expect(
        dead_code,
        reason = "the inbox (track N1) opens a session at the record it lists; remove with its first call"
    )]
    pub(crate) fn reveal(&mut self, seq: EventSeq) {
        self.reveal = Some(seq);
        self.revealed();
    }

    /// The record to reveal, if it is read by now.
    fn revealed(&mut self) {
        let Some(seq) = self.reveal else {
            return;
        };
        let entries = self.timeline.entries();
        let index = match entries.first() {
            None => return,
            // Pruned: the log starts after it.
            Some(first) if first.seq > seq => 0,
            Some(_) => {
                // The entry the record made, or extended (its first record
                // is earlier).
                match entries.iter().rposition(|entry| entry.seq <= seq) {
                    Some(index) if self.timeline.next() > seq => index,
                    // Not read this far yet.
                    Some(_) | None => return,
                }
            }
        };
        self.reveal = None;
        if let Some(row) = self.rows.row_index(index, self.folds.as_ref(), &self.logs) {
            self.list.reveal(row, Reveal::Top);
        }
    }

    /// Fold in `records` of `session`, when it is the session shown:
    /// whether a row changed (the list is told which).
    pub(crate) fn take(&mut self, session: &SessionId, records: &[EventRecord]) -> bool {
        if self
            .shown
            .as_ref()
            .is_none_or(|shown| shown.session != *session)
        {
            return false;
        }
        let rows_before = self.rows.len();
        let change = self.timeline.apply(records);
        if change.is_empty(self.timeline.entries().len()) {
            return false;
        }
        if self.folds.is_some() {
            self.folds = Some(Folds::of(self.timeline.entries()));
        }
        self.logs = Logs::of(self.timeline.entries());
        self.outline = outline(self.timeline.entries());
        self.rows_again();
        let rows_after = self.rows.len();
        // Rows are entries less folded members, so a batch can add rows
        // only at the end and change rows anywhere; measure again from the
        // first row it touched.
        let first_changed = change
            .changed
            .first()
            .copied()
            .unwrap_or(change.added_from)
            .min(change.added_from);
        let from = self
            .rows
            .row_index(first_changed, self.folds.as_ref(), &self.logs)
            .unwrap_or(rows_before)
            .min(rows_before);
        self.list
            .splice(from..rows_before, rows_after.saturating_sub(from));
        self.revealed();
        true
    }

    /// Open or close a run of tool calls.
    fn toggle_run(&mut self, lead: usize) {
        if !self.open_runs.remove(&lead) {
            let _new = self.open_runs.insert(lead);
        }
        let before = self.rows.len();
        self.rows_again();
        let after = self.rows.len();
        // Only the run's own rows change: its lead's row stays where it
        // was and its members come or go right after it. Splicing there
        // (not resetting the list) keeps the view where the person is.
        match self.rows.row_index(lead, self.folds.as_ref(), &self.logs) {
            Some(row) => {
                let next = row.saturating_add(1);
                let gone = before.saturating_sub(after);
                self.list.splice(
                    next..next.saturating_add(gone),
                    after.saturating_sub(before),
                );
                self.list.changed(row..next);
            }
            None => self.list.reset(after),
        }
    }

    /// Open or close a run of the agent's log lines: only its own row
    /// changes (the lines show in its box, never as rows).
    fn toggle_log(&mut self, lead: usize) {
        if !self.open_logs.remove(&lead) {
            let _new = self.open_logs.insert(lead);
        }
        if let Some(row) = self.rows.row_index(lead, self.folds.as_ref(), &self.logs) {
            self.list.changed(row..row.saturating_add(1));
        }
    }

    /// Open or close one entry's disclosure (a tool card, a thought).
    fn toggle_entry(&mut self, entry: usize) {
        if !self.toggled.remove(&entry) {
            let _new = self.toggled.insert(entry);
        }
        if let Some(row) = self.rows.row_index(entry, self.folds.as_ref(), &self.logs) {
            self.list.changed(row..row.saturating_add(1));
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
        self.areas.conversation.open(machine, session);
        for act in [Act::Info, Act::Conversation] {
            self.command(Command::Act {
                machine: machine.to_owned(),
                session,
                act,
            });
        }
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
        if self.areas.conversation.take(session, records) {
            self.find_grew(cx);
        }
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
            Ok(Acted::Done) => {
                area.refused = None;
                // An answer went: what still waits is read again, so a
                // request drawn from the node's word goes once settled.
                if matches!(answer.kind, ActKind::Permit | ActKind::Answer) {
                    self.act(Act::Conversation);
                }
            }
            Ok(Acted::Conversation(held)) => {
                if area
                    .shown
                    .as_ref()
                    .is_some_and(|shown| shown.session == answer.session)
                {
                    area.live = Live::of(&held);
                }
            }
            Err(failed) => area.refused = Some((answer.kind, failed.message)),
        }
        cx.notify();
    }

    /// Ask the node to act on the session shown.
    pub(crate) fn act(&self, act: Act) {
        self.act_at(Seat::SESSION, act);
    }

    /// Ask the node to act on the session the conversation at `seat`
    /// shows.
    pub(crate) fn act_at(&self, seat: Seat, act: Act) {
        if let Some(shown) = (seat.get)(self).and_then(|area| area.shown.as_ref()) {
            self.command(Command::Act {
                machine: shown.machine.clone(),
                session: shown.session,
                act,
            });
        }
    }

    /// Bring record `seq` of the session shown into view (the inbox
    /// opening a session at what it lists; call it after the session area
    /// opened the session).
    #[expect(
        dead_code,
        reason = "the inbox (track N1) opens a session at the record it lists; remove with its first call"
    )]
    pub(crate) fn conversation_reveal(&mut self, seq: EventSeq, cx: &mut Context<'_, Self>) {
        self.areas.conversation.reveal(seq);
        cx.notify();
    }

    /// Open or close a run of tool calls in the conversation at `seat`.
    pub(crate) fn toggle_run(&mut self, seat: Seat, lead: usize, cx: &mut Context<'_, Self>) {
        if let Some(area) = (seat.get_mut)(self) {
            area.toggle_run(lead);
            cx.notify();
        }
    }

    /// Open or close a run of the agent's log lines in the conversation
    /// at `seat`.
    pub(crate) fn toggle_log(&mut self, seat: Seat, lead: usize, cx: &mut Context<'_, Self>) {
        if let Some(area) = (seat.get_mut)(self) {
            area.toggle_log(lead);
            cx.notify();
        }
    }

    /// Open or close one entry's disclosure in the conversation at `seat`.
    pub(crate) fn toggle_entry(&mut self, seat: Seat, entry: usize, cx: &mut Context<'_, Self>) {
        if let Some(area) = (seat.get_mut)(self) {
            area.toggle_entry(entry);
            cx.notify();
        }
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
