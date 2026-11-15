//! What comes into the conversation from the node: an act's answer
//! ([`ConversationArea::answered`]) and a batch of the session's records
//! ([`ConversationArea::take`]), each applied only when it is for the
//! session shown.

use vessel_app_link::{ActAnswer, ActFailed, ActKind, Acted};
use vessel_app_view::timeline::{Folds, Live, Logs, outline};
use vessel_proto::parking::Parked;
use vessel_proto::{EventRecord, SessionEvent, SessionId};

use super::ConversationArea;
use super::area::Followup;

impl ConversationArea {
    /// What an act's answer does to the conversation's own state, only
    /// when it is for the session shown now: a late answer for a session
    /// the person has left, or one that arrives after the machine's
    /// connection was lost, changes nothing here (track V1's review: it
    /// cleared the shown session's lost connection, put another
    /// session's refusal in its bar, and re-read it).
    ///
    /// `lost` is not cleared by an answer: an act answered does not mean
    /// the session's records flow again; records arriving do
    /// ([`Self::take`]).
    pub(crate) fn answered(&mut self, answer: ActAnswer) -> Followup {
        let for_shown = self.shown.as_ref().is_some_and(|shown| {
            shown.machine == answer.machine && shown.session == answer.session
        });
        if !for_shown || self.read_answered(&answer) {
            return Followup::None;
        }
        if matches!(answer.kind, ActKind::StartAgain | ActKind::Wake) {
            self.restarting = false;
            self.again_refused = None;
            // The node's refusal is said under the ended banner, in the
            // words for it; a lost connection stays the bar's.
            if let Err(ActFailed {
                code: code @ Some(_),
                message,
            }) = &answer.result
            {
                self.again_refused = Some((*code, message.clone()));
                return Followup::None;
            }
        }
        // A refused rewind or fork is the notice's (Paseo's toast), said
        // by `rewind` and `fork`, not a refusal beside a control.
        if matches!(answer.kind, ActKind::Rewind | ActKind::Fork)
            && matches!(answer.result, Err(ActFailed { code: Some(_), .. }))
        {
            return Followup::None;
        }
        match answer.result {
            Ok(Acted::Session(info)) => {
                self.refused = None;
                self.info = Some(info);
            }
            Ok(Acted::Deleted(_)) => {
                self.refused = None;
                self.info = None;
            }
            // The new session's; the one shown stands as it was.
            Ok(Acted::Forked(_)) => self.refused = None,
            // Another session's log (a fork's origin, `origin`).
            Ok(Acted::Log(_)) => {}
            Ok(Acted::Done) => {
                self.refused = None;
                // An answer went: what still waits is read again, so a
                // request drawn from the node's word goes once settled.
                if matches!(answer.kind, ActKind::Permit | ActKind::Answer) {
                    return Followup::ReadWaiting;
                }
            }
            Ok(Acted::Conversation(held)) => self.live = Live::of(&held),
            // No code: the node never answered (the connection failed), so
            // it is the session's lost connection, said once in the bar,
            // not a refusal beside a control.
            Err(ActFailed {
                code: None,
                message,
            }) => self.lost = Some(message),
            Err(failed) => self.refused = Some((answer.kind, failed.message)),
        }
        Followup::None
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
        // Records arriving for this session: the connection is back.
        self.lost = None;
        self.follow_parking(records);
        let rows_before = self.rows.len();
        let change = self.timeline.apply(records);
        if change.is_empty(self.timeline.entries().len()) {
            return false;
        }
        self.tail_took(change.added_from);
        self.origin_seen(change.added_from);
        // A batch after the first read: what it brings arrives on screen.
        if change.added_from > 0 {
            self.fresh = Some((change.added_from, std::time::Instant::now()));
        }
        if self.folds.is_some() {
            self.folds = Some(Folds::of(self.timeline.entries()));
        }
        self.logs = Logs::of(self.timeline.entries());
        self.outline = outline(self.timeline.entries());
        // The rail lists the conversation's prompts: not what a rewind
        // left behind.
        let timeline = &self.timeline;
        self.outline.retain(|item| !timeline.rewound(item.entry));
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
        self.list.splice(
            self.at(from)..self.at(rows_before),
            rows_after.saturating_sub(from),
        );
        // A fork that never held a conversation shows no origin (F2).
        self.origin_dropped();
        self.revealed();
        true
    }

    /// The session's own `parked` and `woken` records keep the session's
    /// description current, so the bar and the banner say released or not
    /// as the timeline does (the description is read only when the
    /// session opens and after an act). In order: a log read from its
    /// start ends where the session stands.
    fn follow_parking(&mut self, records: &[EventRecord]) {
        let Some(info) = self.info.as_mut() else {
            return;
        };
        for record in records {
            if let SessionEvent::Parked { reason, .. } = &record.event {
                info.parked = Some(Parked {
                    since: record.at,
                    reason: reason.clone(),
                });
            } else if matches!(record.event, SessionEvent::Woken) {
                info.parked = None;
            }
        }
    }
}
