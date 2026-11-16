//! What the shell tells the conversation: a session opened, Settings'
//! display, the session's records, an act's answer, a lost connection, a
//! record to bring into view, a disclosure turned, and the machine book's
//! renames and forgets.

use vessel_app_kit::ui::Context;
use vessel_app_link::{Act, ActAnswer, ActKind, Acted, Command};
use vessel_app_view::Card;
use vessel_app_view::prefs::ConversationDisplay;
use vessel_app_view::session::is_live;
use vessel_app_view::timeline::Folds;
use vessel_proto::{EventRecord, EventSeq, SessionId};

use super::Seat;
use super::area::{Followup, collapsing};
use crate::areas::session::Open;
use crate::shell::Shell;

impl Shell {
    /// The session area opened a session: start again from its first
    /// record, and read what the node says of it. A terminal session has
    /// no conversation state (the node refuses `Act::Conversation` for one
    /// and the refusal showed as the screen's notice), so only its info is
    /// read.
    pub(crate) fn conversation_opened(&mut self, open: &Open, cx: &mut Context<'_, Self>) {
        self.hear_find(cx);
        self.areas.conversation.open(&open.machine, open.session);
        // Opened again: a fork's origin that could not be read is read.
        self.origin_read();
        let mut acts = vec![Act::Info];
        if open.structured {
            acts.push(Act::Conversation);
        }
        for act in acts {
            self.command(Command::Act {
                machine: open.machine.clone(),
                session: open.session,
                act,
            });
        }
    }

    /// The conversation as Settings' display says: its runs folded or
    /// not, with the reading anchor preserved while rows are measured again.
    pub(crate) fn conversation_display(&mut self, display: ConversationDisplay) {
        let collapse = collapsing(Some(display));
        let area = &mut self.areas.conversation;
        let turn_collapse = super::turns::enabled(Some(display));
        if area.collapse == collapse && area.turns.is_some() == turn_collapse {
            return;
        }
        let bookmark = area.reading_bookmark();
        area.collapse = collapse;
        area.turns = turn_collapse.then(super::turns::Turns::default);
        area.folds = collapse
            .then(|| Folds::with_reasoning(area.timeline.entries(), area.reasoning.expanded()));
        area.rows_again();
        self.origin_display(collapse);
        let area = &mut self.areas.conversation;
        area.list.reset(area.at(area.rows.len()));
        area.tail_dropped();
        area.restore_reading(bookmark);
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
            // A fork's `forked` line came: its origin is read (`origin`).
            self.origin_read();
        }
    }

    /// A session act answered. The fleet's card and the frame's notice
    /// follow every answer; what the answer does to the conversation's own
    /// state happens only when it is for the session shown
    /// ([`ConversationArea::answered`]).
    pub(crate) fn acted(&mut self, answer: ActAnswer, cx: &mut Context<'_, Self>) {
        // A terminal session's log, read whole (`terminal`).
        if self.terminal_acted(&answer, cx) {
            cx.notify();
            return;
        }
        match &answer.result {
            Ok(Acted::Session(info)) => {
                let card = Card::from_session(&answer.machine, info);
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
            }
            Ok(Acted::Deleted(deleted)) => {
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
            // A fork's new session is listed at once (the list's next
            // read would list it too).
            Ok(Acted::Forked(info)) => self.fleet.add_session(&answer.machine, info),
            Ok(Acted::Done | Acted::Conversation(_) | Acted::Log(_)) | Err(_) => {}
        }
        // A fork's origin read (its answer is for the origin, not the
        // session shown).
        self.origin_answered(&answer, cx);
        // Started again: the session area follows it afresh (the node ends
        // a follow at the final status), as a send to an ended session
        // does.
        let again = (answer.kind == ActKind::StartAgain
            && matches!(&answer.result, Ok(Acted::Session(info)) if is_live(info))
            && self.areas.conversation.shown.as_ref().is_some_and(|shown| {
                shown.machine == answer.machine && shown.session == answer.session
            }))
        .then_some(answer.session);
        // A rewind's answer for the session shown lifts its menu's lock
        // and puts the message back (`rewind`).
        if self
            .areas
            .conversation
            .shown
            .as_ref()
            .is_some_and(|shown| shown.machine == answer.machine && shown.session == answer.session)
        {
            self.rewind_answered(&answer);
            self.fork_answered(&answer);
            self.empty_fork_removed(&answer);
        }
        if self.areas.conversation.answered(answer) == Followup::ReadWaiting {
            self.act(Act::Conversation);
        }
        if let Some(session) = again {
            self.resumed(session, cx);
        }
        cx.notify();
    }

    /// The session shown lost its machine's connection (see `lost`).
    pub(crate) fn session_lost(&mut self, session: SessionId, why: String) {
        let area = &mut self.areas.conversation;
        if area
            .shown
            .as_ref()
            .is_some_and(|shown| shown.session == session)
        {
            area.lost = Some(why);
        }
    }

    /// Bring record `seq` of the session shown into view (the inbox
    /// opening a session at what it lists; call it after the session area
    /// opened the session).
    pub(crate) fn conversation_reveal(&mut self, seq: EventSeq, cx: &mut Context<'_, Self>) {
        self.areas.conversation.reveal(seq);
        cx.notify();
    }

    /// Open or close a run of tool calls in the conversation at `seat`.
    pub(crate) fn toggle_run(&mut self, seat: &Seat, lead: usize, cx: &mut Context<'_, Self>) {
        let before = seat.origin_height(self);
        if let Some(area) = seat.get_mut(self) {
            area.toggle_run(lead);
            self.origin_toggled(seat, before);
            cx.notify();
        }
    }

    /// Open or close one entry's disclosure in the conversation at `seat`.
    pub(crate) fn toggle_entry(&mut self, seat: &Seat, entry: usize, cx: &mut Context<'_, Self>) {
        let before = seat.origin_height(self);
        if let Some(area) = seat.get_mut(self) {
            area.toggle_entry(entry);
            self.origin_toggled(seat, before);
            cx.notify();
        }
    }

    /// The node book renamed machine `from` to `to` (Settings › Machines,
    /// once the book took the edit; a failed edit calls nothing): the
    /// session shown on it follows the name. Nothing is asked of the
    /// machine.
    #[expect(
        dead_code,
        reason = "Settings › Machines (track E2) calls it; remove with its first call"
    )]
    pub(crate) fn conversation_machine_renamed(
        &mut self,
        from: &str,
        to: &str,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(shown) = self.areas.conversation.shown.as_mut()
            && shown.machine_renamed(from, to)
        {
            cx.notify();
        }
    }

    /// The node book forgot machine `name` (once the book took it): a
    /// session shown on it is closed here, its state dropped. Nothing is
    /// stopped or deleted on the machine.
    #[expect(
        dead_code,
        reason = "Settings › Machines (track E2) calls it; remove with its first call"
    )]
    pub(crate) fn conversation_machine_forgotten(
        &mut self,
        name: &str,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.conversation;
        if area.shown.as_ref().is_some_and(|shown| shown.on(name)) {
            area.clear();
            cx.notify();
        }
    }
}
