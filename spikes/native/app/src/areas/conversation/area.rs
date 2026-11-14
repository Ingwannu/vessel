//! The conversation area's state ([`ConversationArea`]): the session
//! shown, its timeline folded from the node's records, the rows drawn and
//! their list, and every part of the frame's own state; what an act's
//! answer does to it, and folding in a batch of records.

use std::collections::HashSet;
use std::rc::Rc;

use vessel_app_kit::ui::{App, Subscription};
use vessel_app_kit::{DialogState, MenuState, Reveal, VirtualList};
use vessel_app_link::ActKind;
use vessel_app_view::prefs::ConversationDisplay;
use vessel_app_view::timeline::{FindCache, Folds, Live, Logs, OutlineItem, Rows, Timeline};
use vessel_proto::{ErrorCode, EventSeq, SessionId, SessionInfo};

use super::actions::{AgentLog, Confirm, MenuAction};
use super::{
    bench, find, fork, meter, origin, outline, permission, question, queue, read, rename, rewind,
    search, tail,
};

/// What the shell does after an act's answer reached the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Followup {
    /// Nothing more.
    None,
    /// Read again what waits on the session (an answer went).
    ReadWaiting,
}

/// The session this area shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shown {
    pub(crate) machine: String,
    pub(crate) session: SessionId,
}

impl Shown {
    /// The node book renamed machine `from` to `to`: a session on it
    /// follows the name (its acts go to the machine by its new name).
    /// Whether it was on it.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Settings › Machines (track E2) calls the hook; remove with its first call"
        )
    )]
    pub(super) fn machine_renamed(&mut self, from: &str, to: &str) -> bool {
        if self.machine != from {
            return false;
        }
        to.clone_into(&mut self.machine);
        true
    }

    /// Whether the session is on machine `name`.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Settings › Machines (track E2) calls the hook; remove with its first call"
        )
    )]
    pub(super) fn on(&self, name: &str) -> bool {
        self.machine == name
    }
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
    /// Whether the agent's log is shown (the Session actions' "Agent
    /// log").
    pub(crate) agent_log: AgentLog,
    /// Whether tool calls are collapsed (Settings' "Collapse tool calls"
    /// against "Full"); see [`collapsing`].
    pub(super) collapse: bool,
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
    /// Start again, or a wake after a failed resume, is on its way (the
    /// banner's button, the menu's row): none is sent a second time.
    pub(super) restarting: bool,
    /// The node refused to start the session again: its code and words,
    /// said under the ended banner.
    pub(super) again_refused: Option<(Option<ErrorCode>, String)>,
    /// The palette's message search (kept when another session shows).
    pub(crate) search: search::Searching,
    /// The rows after the entries: the message sent and not yet
    /// recorded, the working footer.
    pub(super) tail: tail::Tail,
    /// The first entry of the last batch that came while the session was
    /// shown, and when: its replies grow in (`motion`).
    pub(super) fresh: Option<(usize, std::time::Instant)>,
    /// The session's machine is not connected (records stopped, or an act,
    /// a prompt or an input found no connection): the transport's words.
    /// Said once, in the bar, in the app's words; these only in the line's
    /// tooltip (`bar.rs`). Not a refusal: the node did not answer at all.
    pub(crate) lost: Option<String>,
    /// The tooltip of that line and where it is drawn.
    pub(crate) lost_tip: (vessel_app_kit::Tooltips, vessel_app_kit::tooltip::Place),
    /// The tooltip of the bar's word for a released agent.
    pub(crate) parked_tip: (vessel_app_kit::Tooltips, vessel_app_kit::tooltip::Place),
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
    /// The context meter left of the send button, and its card.
    pub(crate) meter: meter::MeterState,
    /// Marking the session read while it is in front.
    pub(super) read: read::ReadMarks,
    /// A rewind being chosen under a person's message, or sent.
    pub(super) rewinding: rewind::Rewinding,
    /// A fork being chosen in a turn's footer, sent, or opening.
    pub(super) forking: fork::Forking,
    /// A fork's origin, its turns the list's first rows (`origin`).
    pub(crate) origin: origin::Origin,
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
    pub(super) reveal: Option<EventSeq>,
    /// The find bar, while it is open.
    pub(crate) find: Option<find::FindBar>,
    /// Each entry's folded text for find, kept between searches of the
    /// session shown.
    pub(crate) find_cache: FindCache,
    /// Hears Mod+F (`find`); made once a session is first shown.
    pub(super) keys: Option<Subscription>,
    /// The scrolling measurement, when it is on.
    pub(super) bench: bench::Bench,
}

impl ConversationArea {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            shown: None,
            timeline: Timeline::default(),
            folds: None,
            open_runs: HashSet::new(),
            logs: Logs::default(),
            agent_log: AgentLog::Hidden,
            collapse: collapsing(None),
            toggled: HashSet::new(),
            rows: Rows::All(0),
            list: VirtualList::new(true),
            info: None,
            refused: None,
            restarting: false,
            again_refused: None,
            search: search::Searching::default(),
            tail: tail::Tail::default(),
            fresh: None,
            lost: None,
            lost_tip: (
                vessel_app_kit::Tooltips::new(),
                Rc::new(std::cell::Cell::new(None)),
            ),
            parked_tip: (
                vessel_app_kit::Tooltips::new(),
                Rc::new(std::cell::Cell::new(None)),
            ),
            menu: MenuState::new(cx),
            menu_rows: Vec::new(),
            confirm: None,
            dialog: DialogState::new(cx),
            rename: None,
            queue_edit: None,
            answering: None,
            meter: meter::MeterState::default(),
            read: read::ReadMarks::default(),
            rewinding: rewind::Rewinding::new(cx),
            forking: fork::Forking::new(cx),
            origin: origin::Origin::None,
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
        // The session shown opened again (started again after it ended,
        // or chosen again): what it shows stays, and the records read
        // again from its start add only what is new (Paseo's agent reopens
        // its conversation; nothing is reloaded).
        if self
            .shown
            .as_ref()
            .is_some_and(|shown| shown.machine == machine && shown.session == session)
        {
            // A fork's origin that could not be read is asked for again.
            self.origin_again();
            return;
        }
        self.clear();
        self.shown = Some(Shown {
            machine: machine.to_owned(),
            session,
        });
    }

    /// Show nothing: what was shown is dropped, here only.
    pub(super) fn clear(&mut self) {
        self.shown = None;
        self.lost = None;
        self.read = read::ReadMarks::default();
        self.rewinding.clear();
        self.forking.clear();
        self.origin = origin::Origin::None;
        self.timeline = Timeline::default();
        self.folds = self.collapse.then(Folds::default);
        self.open_runs.clear();
        self.logs = Logs::default();
        self.agent_log = AgentLog::Hidden;
        self.toggled.clear();
        self.rows = Rows::All(0);
        self.list.reset(0);
        self.tail = tail::Tail::default();
        self.fresh = None;
        self.info = None;
        self.refused = None;
        self.restarting = false;
        self.again_refused = None;
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
    pub(crate) fn reveal(&mut self, seq: EventSeq) {
        self.reveal = Some(seq);
        self.revealed();
    }

    /// The record to reveal, if it is read by now.
    pub(super) fn revealed(&mut self) {
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
            self.list.reveal(self.at(row), Reveal::Top);
        }
    }

    /// Open or close a run of tool calls.
    pub(super) fn toggle_run(&mut self, lead: usize) {
        if !self.open_runs.remove(&lead) {
            let _new = self.open_runs.insert(lead);
        }
        let before = self.rows.len();
        self.rows_again();
        let after = self.rows.len();
        // Only the run's own rows change: its lead's row stays where it
        // was and its members come or go right after it. Splicing there
        // (not resetting the list) keeps the view where the person is.
        if let Some(row) = self.rows.row_index(lead, self.folds.as_ref(), &self.logs) {
            let (row, next) = (self.at(row), self.at(row.saturating_add(1)));
            let gone = before.saturating_sub(after);
            self.list.splice(
                next..next.saturating_add(gone),
                after.saturating_sub(before),
            );
            self.list.changed(row..next);
        } else {
            self.list.reset(self.at(after));
            self.tail_dropped();
        }
    }

    /// Open or close one entry's disclosure (a tool card, a thought).
    pub(super) fn toggle_entry(&mut self, entry: usize) {
        if !self.toggled.remove(&entry) {
            let _new = self.toggled.insert(entry);
        }
        if let Some(row) = self.rows.row_index(entry, self.folds.as_ref(), &self.logs) {
            let row = self.at(row);
            self.list.changed(row..row.saturating_add(1));
        }
    }

    /// Whether a permission request waits for the person's answer: one in
    /// the log read, or one the node says waits whose record the log no
    /// longer holds (the card over the composer shows either). Send's
    /// Queue interrupts behind it (`view::session::send_delivery`).
    pub(crate) fn permission_waits(&self) -> bool {
        self.timeline.waiting_permissions() > 0
            || !self.timeline.unseen_permissions(&self.live).is_empty()
    }

    /// The rows again, after the entries, the folds or the open runs
    /// changed.
    pub(super) fn rows_again(&mut self) {
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
pub(super) fn collapsing(display: Option<ConversationDisplay>) -> bool {
    match std::env::var("VESSEL_APP_CONVERSATION").as_deref() {
        Ok("full") => false,
        Ok("collapse") => true,
        _ => display.is_none_or(|display| display == ConversationDisplay::CollapseTools),
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::SessionId;

    use super::Shown;

    fn shown(machine: &str) -> Shown {
        Shown {
            machine: machine.to_owned(),
            session: SessionId::random(),
        }
    }

    #[test]
    fn a_rename_moves_the_session_on_that_machine_to_the_new_name() {
        let mut on_laptop = shown("laptop");
        let session = on_laptop.session;
        assert!(on_laptop.machine_renamed("laptop", "work-laptop"));
        assert_eq!(on_laptop.machine, "work-laptop");
        assert_eq!(on_laptop.session, session);
        // Acts now go to the new name; the old one is no longer this one's.
        assert!(on_laptop.on("work-laptop"));
        assert!(!on_laptop.on("laptop"));
    }

    #[test]
    fn a_rename_of_another_machine_leaves_the_session_alone() {
        let mut on_server = shown("build-server");
        assert!(!on_server.machine_renamed("laptop", "work-laptop"));
        assert_eq!(on_server.machine, "build-server");
    }

    #[test]
    fn a_forget_closes_only_a_session_on_that_machine() {
        assert!(shown("laptop").on("laptop"));
        assert!(!shown("build-server").on("laptop"));
    }
}
