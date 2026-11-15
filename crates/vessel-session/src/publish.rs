//! [`Publisher`]: numbers events and delivers them to the store and to
//! followers, and keeps the session's info current, its place on the board
//! included. Used only by the session actor.

use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};

use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, watch};
use vessel_proto::{
    AgentAbilities, AgentEvent, EventRecord, EventSeq, HarnessSessionId, ProfileName, SessionBoard,
    SessionEvent, SessionId, SessionInfo, SessionOption, SessionStatus, SessionTitle,
};

use crate::activity::Tracker;
use crate::conversation::Seen;
use crate::forge::ForgeView;
use crate::handle::Shared;

/// The actor's write side of a session.
///
/// Publishing is two steps so the actor can stay responsive while the store
/// is slow: [`slot`](Self::slot) waits for room in the durable channel
/// (cancel-safe, so it can sit in a `select!` next to `kill`), and
/// [`commit`](Self::commit) then numbers the event and delivers it
/// everywhere without waiting.
#[derive(Debug)]
pub(crate) struct Publisher {
    shared: Arc<Shared>,
    /// `None` once the node dropped its receiver.
    durable: Option<mpsc::Sender<Arc<EventRecord>>>,
    info: watch::Sender<SessionInfo>,
    next: EventSeq,
    /// What the agent is doing, read off every committed record; the
    /// source of `SessionInfo::board`.
    activity: Tracker,
}

/// Room for one record in the durable channel.
#[derive(Debug)]
pub(crate) enum Slot {
    Durable(mpsc::OwnedPermit<Arc<EventRecord>>),
    /// The node no longer receives records (it dropped the channel).
    Gone,
}

impl Publisher {
    /// Updated only once a new agent successfully starts; publish the
    /// profile event afterward so its readers see the matching binding.
    pub(crate) fn set_limit_association(&self, association: crate::LimitAssociation) {
        let _previous = self.shared.limits.send_replace(association);
    }

    /// `activity` must be the tracker `info`'s first board came from, so
    /// the board changes only with what the session publishes. The first
    /// record is numbered `info`'s `next_seq`: zero for a new session, the
    /// record after its stored log for one started again (protocol 10).
    pub(crate) fn new(
        shared: Arc<Shared>,
        durable: mpsc::Sender<Arc<EventRecord>>,
        info: watch::Sender<SessionInfo>,
        activity: Tracker,
    ) -> Self {
        let next = info.borrow().next_seq;
        Self {
            shared,
            durable: Some(durable),
            info,
            next,
            activity,
        }
    }

    /// Wait for room in the durable channel. Cancel-safe.
    pub(crate) async fn slot(&self) -> Slot {
        let Some(durable) = &self.durable else {
            return Slot::Gone;
        };
        // An owned permit (on a cloned sender, one atomic increment) so the
        // permit does not borrow `self` while `commit` mutates it.
        match durable.clone().reserve_owned().await {
            Ok(permit) => Slot::Durable(permit),
            Err(_closed) => Slot::Gone,
        }
    }

    /// The sequence number the next record committed gets.
    pub(crate) const fn next_seq(&self) -> EventSeq {
        self.next
    }

    /// Borrow only during the actor's comparison, before its durable
    /// publish await, so a conditional title change stays in actor order.
    pub(crate) fn current(&self) -> watch::Ref<'_, SessionInfo> {
        self.info.borrow()
    }

    /// The session it publishes for.
    pub(crate) fn session(&self) -> SessionId {
        self.shared.id
    }

    /// Replace the actual run on restart, or clear it when the agent parks.
    pub(crate) fn running(&self, process: Option<vessel_proc::ProcessLifetime>) {
        let pid = process.as_ref().map_or(0, |process| process.pid().get());
        self.shared.pid.store(pid, Ordering::Release);
        *self
            .shared
            .process
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = process;
    }

    /// Whether a permission or a question waits for an answer (protocol
    /// 12: a session waiting on a person is not parked).
    pub(crate) fn waiting_for_answers(&self) -> bool {
        self.shared
            .seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .waiting()
    }

    /// Number `event`, hand it to the store, append it to the ring,
    /// broadcast it, and update the session info, in that order.
    ///
    /// The info is updated last, so a reader that sees `next_seq = N` can
    /// always `follow` up to `N`. Its board is derived again from the
    /// record (no allocation: the reducer reads a few flags); watchers are
    /// woken only if that moved the lane, the phrase or the activity.
    pub(crate) fn commit(&mut self, slot: Slot, event: SessionEvent) {
        let seq = self.next;
        // The last `u64` is never handed out, so `next_seq` stays
        // representable. Unreachable in practice (2^64 events).
        let Some(next) = seq.next() else {
            tracing::error!(session = %self.shared.id, "event sequence exhausted; dropping event");
            self.shared.kill.cancel();
            return;
        };
        let at = Utc::now();
        let change = InfoChange::of(&event);
        if let SessionEvent::OptionsChanged { options } = &event {
            let reported: Arc<[SessionOption]> = Arc::from(options.as_slice());
            *self
                .shared
                .reported
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(reported);
        }
        self.activity.observe(&event, at);
        if Seen::cares(&event) {
            self.shared
                .seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .observe(&event);
        }
        let board = self.activity.board(at);
        // Before the record is anywhere: whoever sees the end finds it.
        if let SessionEvent::Status { status } = &event
            && !status.is_live()
        {
            self.shared.ended.store(true, Ordering::Release);
            drop(
                self.shared
                    .process
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take(),
            );
        }
        // The one allocation of this record; the store, the ring and every
        // follower share it.
        let record = Arc::new(EventRecord { seq, at, event });
        // Before the record is anywhere too: whoever sees a turn's end
        // finds the session unread.
        if record.event.asks_for_attention() {
            self.shared.marks().asked(seq);
        }
        match slot {
            Slot::Durable(permit) => drop(permit.send(Arc::clone(&record))),
            Slot::Gone => {
                if self.durable.take().is_some() {
                    tracing::warn!(
                        session = %self.shared.id,
                        "durable receiver dropped; records are no longer persisted"
                    );
                }
            }
        }
        self.shared.ring().append(record);
        self.next = next;
        let _notified = self.info.send_if_modified(|info| {
            info.next_seq = next;
            // Both run: neither may skip the other.
            let changed = change.apply(info, at);
            let moved = place(info, board);
            changed || moved
        });
    }

    /// A terminal's output has been quiet (`terminal::QUIET`): the session
    /// is idle now. Not a record: the log has no event for the absence of
    /// output, so only the info changes.
    pub(crate) fn quiet(&mut self) {
        self.activity.quiet();
        let board = self.activity.board(Utc::now());
        let _notified = self.info.send_if_modified(|info| place(info, board));
    }

    /// What the node knows of the session's pull request now (protocol
    /// 11, ADR 0029): its summary in the info, its facts in the board.
    /// Not a record: the log keeps none of a forge (the store places a
    /// stored session without it). Ignored once the session has ended,
    /// whose info has none.
    pub(crate) fn forge(&mut self, view: Option<&ForgeView>) {
        if self.shared.ended.load(Ordering::Acquire) {
            return;
        }
        self.activity.forge(view);
        let board = self.activity.board(Utc::now());
        let summary = view.map(|view| Box::new(view.info.clone()));
        let _notified = self.info.send_if_modified(|info| {
            let changed = info.pull_request != summary;
            info.pull_request = summary;
            let moved = place(info, board);
            changed || moved
        });
    }
}

/// Put `board` in `info`; returns whether watchers should be woken.
///
/// Only a new lane, phrase or activity wakes them. The time of the last
/// activity alone moves silently, as `next_seq` does: it changes with every
/// burst of terminal output, and waking every watcher for it would turn
/// the info watch into an output stream.
fn place(info: &mut SessionInfo, board: SessionBoard) -> bool {
    let wake = info.board.as_ref().is_none_or(|current| {
        current.column != board.column
            || current.display_status != board.display_status
            || current.activity != board.activity
    });
    info.board = Some(board);
    wake
}

/// The session is over once its actor drops the publisher: live followers
/// are told (their receivers close), as are info watchers (the `watch`
/// sender drops with it) and the node (the durable sender drops with it).
impl Drop for Publisher {
    fn drop(&mut self) {
        self.shared.ring().close();
    }
}

/// What an event changes in [`SessionInfo`] besides `next_seq`.
enum InfoChange {
    None,
    Status(SessionStatus),
    /// The agent is ready: its session id, and (protocol 10) what it says
    /// it can do, which the info carries as the store's row does.
    Ready(HarnessSessionId, Option<AgentAbilities>),
    Profile(Option<ProfileName>),
    Title(SessionTitle, vessel_proto::TitleOrigin),
    /// Its agent was parked (`Some`) or woken (protocol 12).
    Parked(Option<vessel_proto::parking::ParkReason>),
}

impl InfoChange {
    fn of(event: &SessionEvent) -> Self {
        match event {
            SessionEvent::Status { status } => Self::Status(status.clone()),
            SessionEvent::Agent {
                event:
                    AgentEvent::Ready {
                        harness_session,
                        abilities,
                    },
            } => Self::Ready(harness_session.clone(), *abilities),
            SessionEvent::ProfileChanged { to, .. } => Self::Profile(to.clone()),
            SessionEvent::Renamed { title, origin } => Self::Title(title.clone(), *origin),
            SessionEvent::Parked { reason } => Self::Parked(Some(reason.clone())),
            SessionEvent::Woken => Self::Parked(None),
            // The agent's conversation from a rewind on (protocol 14): the
            // restarted agent's own `ready` is not logged.
            SessionEvent::Rewound {
                harness_session, ..
            } => Self::Ready(harness_session.clone(), None),
            // Options are not part of the info: a client reads them from
            // the log, or asks `session.options`.
            SessionEvent::Terminal(_)
            | SessionEvent::Agent { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Forked { .. } => Self::None,
        }
    }

    /// Apply to `info`; returns whether watchers should be woken.
    fn apply(self, info: &mut SessionInfo, at: DateTime<Utc>) -> bool {
        match self {
            Self::None => false,
            Self::Status(status) => {
                if !status.is_live() {
                    info.ended_at = Some(at);
                    // Nothing watches an ended session; the store keeps no
                    // pull request, so the two agree (ADR 0029). Nor is an
                    // ended one parked.
                    info.pull_request = None;
                    info.parked = None;
                }
                info.status = status;
                true
            }
            Self::Ready(id, abilities) => {
                info.harness_session = Some(id);
                if abilities.is_some() {
                    info.abilities = abilities;
                }
                true
            }
            Self::Profile(profile) => {
                info.profile = profile;
                true
            }
            Self::Title(title, origin) => {
                info.title = Some(title);
                info.title_origin = origin;
                true
            }
            Self::Parked(reason) => {
                info.parked =
                    reason.map(|reason| vessel_proto::parking::Parked { since: at, reason });
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::{
        Activity, DisplayStatus, HarnessKind, KanbanColumn, SessionId, SessionMode, SessionSpec,
        TerminalSize,
    };

    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap()
    }

    fn board(display_status: DisplayStatus, activity: Activity, seconds: i64) -> SessionBoard {
        SessionBoard::new(
            KanbanColumn::Building,
            display_status,
            activity,
            at(seconds),
        )
    }

    /// Only a new lane, phrase or activity wakes watchers; a later last
    /// activity alone is kept without waking them.
    #[test]
    fn only_a_new_place_wakes_watchers() {
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        );
        let mut info = SessionInfo::new(SessionId::random(), spec, at(0));
        assert!(
            place(
                &mut info,
                board(DisplayStatus::Working, Activity::Active, 0)
            ),
            "the first board"
        );
        assert!(
            !place(
                &mut info,
                board(DisplayStatus::Working, Activity::Active, 5)
            ),
            "more output"
        );
        assert_eq!(
            info.board.as_ref().map(|board| board.last_activity_at),
            Some(at(5)),
            "kept, silently"
        );
        assert!(
            place(&mut info, board(DisplayStatus::Idle, Activity::Idle, 5)),
            "quiet"
        );
        assert!(
            place(
                &mut info,
                SessionBoard::new(
                    KanbanColumn::Archive,
                    DisplayStatus::Terminated,
                    Activity::Exited,
                    at(9)
                )
            ),
            "the end"
        );
    }
}
