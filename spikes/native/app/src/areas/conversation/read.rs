//! Marking the session shown read while it is in front (ADR 0026; the
//! page's `features/session/use-read-marks.ts`): `read { through }`, the
//! record after the last one shown, sent when the window is active and
//! the session has shown something that asks for attention since the
//! last mark (or the list says unread); at most once a second; never
//! right after the person marked it unread here; and no more to a machine
//! that answered `forbidden` (a device given only `read`). The rule is the
//! view's (`view::session::read_through`).
//!
//! [Decision Log]
//! - Purpose: a session read on screen stayed unread in the list and on
//!   the board: the view's rule was written and tested, and nothing
//!   called it.
//! - Constraint: whether the window is in front is known while drawing
//!   (`Window::is_window_active`), not where records arrive; GPUI draws
//!   again when the window gains or loses focus and when records arrive.
//! - Choice: the timeline's draw asks the rule (cheap, a few comparisons)
//!   and, when a mark is due, defers the send to after the frame; a mark
//!   due within the second after the last waits for the rest of it and
//!   draws again. A read mark this area sent and the node refused stays
//!   quiet (the page's): it is not the person's action, so no refusal is
//!   shown beside a control.
//! - Cost: a read mark and the person's pin or archive answered at the
//!   same moment are told apart by kind only (both are marks); a pin's
//!   refusal arriving while a read is on its way is taken as the read's
//!   and not shown.
//! - Inbox read (claims row 4): on a machine that admits ADR 0089's
//!   protocol, the same read goes as `inbox.read` (`read_inbox_through`),
//!   which takes the `Read` scope, so a device given only `read` marks too.
//!   Nothing answers it but the session's list, so nothing waits in
//!   flight. While the protocol is unassigned no machine admits it, and the
//!   read mark stays as it is (`view::notices::claims::read_call`). The
//!   person's own "Mark as read" in a row's menu stays a mark.

use std::cell::Cell;
use std::time::{Duration, Instant};

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::claims::ReadInbox;
use vessel_app_link::{Act, ActAnswer, ActFailed, ActKind, Command};
use vessel_app_view::notices::claims::{ReadCall, read_call};
use vessel_app_view::session::{ReadView, read_through};
use vessel_proto::{ErrorCode, EventSeq, Mark, SessionId};

use super::{ConversationArea, Seat};
use crate::shell::Shell;

/// At most one read mark a second (the page's).
const GAP: Duration = Duration::from_secs(1);

/// What the session shown knows about its read marks.
#[derive(Debug, Default)]
pub(crate) struct ReadMarks {
    /// The last `through` sent.
    sent: Option<EventSeq>,
    /// When it was sent.
    last: Option<Instant>,
    /// The record after the last one shown when the person marked it
    /// unread here: no read until a record comes after it.
    held_at: Option<EventSeq>,
    /// The machine refused a read mark: none goes to it again.
    read_only: bool,
    /// A read mark is on its way.
    in_flight: bool,
    /// A check is deferred to after the frame (set while drawing).
    scheduled: Cell<bool>,
}

impl ConversationArea {
    /// The mark due now, by the view's rule.
    fn read_due(&self, front: bool) -> Option<EventSeq> {
        let info = self.info.as_ref()?;
        if self.read.read_only || self.read.in_flight {
            return None;
        }
        read_through(&ReadView {
            front,
            unread: info.marks.unread,
            attention: self.timeline.attention(),
            next: self.timeline.next(),
            sent: self.read.sent,
            held_at: self.read.held_at,
        })
    }

    /// The person marked the session unread: the open screen does not
    /// mark it read again until a record comes after what it shows.
    fn read_held(&mut self) {
        self.read.held_at = Some(self.timeline.next());
    }

    /// The answer to a read mark this area sent, when `answer` is one: it
    /// is settled here and nothing is said of a refusal. `false`: not one.
    pub(super) fn read_answered(&mut self, answer: &ActAnswer) -> bool {
        if answer.kind != ActKind::Mark || !self.read.in_flight {
            return false;
        }
        self.read.in_flight = false;
        tracing::debug!(ok = answer.result.is_ok(), "the read mark answered");
        match &answer.result {
            Ok(_) => false,
            Err(ActFailed {
                code: Some(ErrorCode::Forbidden),
                ..
            }) => {
                self.read.read_only = true;
                true
            }
            Err(_) => true,
        }
    }
}

/// Called while the timeline is drawn: when a read mark is due, check
/// again after the frame and send it.
pub(super) fn check(shell: &Shell, window: &Window, cx: &mut Context<'_, Shell>) {
    let area = &shell.areas.conversation;
    if area.read.scheduled.get() || area.read_due(window.is_window_active()).is_none() {
        return;
    }
    area.read.scheduled.set(true);
    cx.defer_in(window, |shell, window, cx| {
        shell.conversation_read(window, cx);
    });
}

impl Shell {
    /// Session `session` on `machine` was marked unread in this app (the
    /// Session actions menu here, a sidebar row's, a board card's; call it
    /// as the mark is sent): when it is the session shown, the open screen
    /// does not mark it read again until a record comes after what it
    /// shows, so the person's mark stays. Another session: nothing.
    pub(crate) fn conversation_marked_unread(&mut self, machine: &str, session: SessionId) {
        let area = &mut self.areas.conversation;
        if area
            .shown
            .as_ref()
            .is_some_and(|shown| shown.machine == machine && shown.session == session)
        {
            area.read_held();
        }
    }

    /// Send the read mark due, or wait for the rest of the second.
    fn conversation_read(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        area.read.scheduled.set(false);
        let Some(through) = area.read_due(window.is_window_active()) else {
            return;
        };
        let since = area.read.last.map(|last| last.elapsed());
        if let Some(wait) = since.and_then(|since| GAP.checked_sub(since)) {
            area.read.scheduled.set(true);
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(wait).await;
                let _gone = this.update_in(cx, |shell, window, cx| {
                    shell.conversation_read(window, cx);
                });
            })
            .detach();
            return;
        }
        area.read.sent = Some(through);
        area.read.last = Some(Instant::now());
        area.read.held_at = None;
        tracing::debug!(%through, "the session shown marked read");
        if let Some(read) = self.inbox_read(through) {
            // `inbox.read` is answered by the session's list, not by an
            // act's answer, so nothing waits on it here.
            self.command(Command::ReadInbox(read));
            return;
        }
        self.areas.conversation.read.in_flight = true;
        self.act(Act::Mark(Mark::Read { through }));
    }

    /// The `inbox.read` for the session shown, when its machine admits ADR
    /// 0089's protocol (claims row 4). Below it, the read mark stays as it
    /// is (`session.mark`).
    fn inbox_read(&self, through: EventSeq) -> Option<ReadInbox> {
        // The session `act` would mark: the same seat's target.
        let shown = Seat::SESSION.legacy_target(self)?;
        let protocol = self
            .fleet
            .machines
            .iter()
            .find(|known| known.name == shown.machine)
            .and_then(|known| known.protocol);
        (read_call(protocol) == ReadCall::Inbox).then(|| ReadInbox {
            machine: shown.machine.clone(),
            session: shown.session,
            through,
        })
    }
}

// Owner snapshot preparation; no staged exemption in the product path.
#[cfg(test)]
impl ReadMarks {
    pub(super) fn snapshot_busy(&self) -> bool {
        self.in_flight || self.scheduled.get()
    }
}

#[cfg(test)]
mod tests;
