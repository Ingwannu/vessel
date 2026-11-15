//! What follows the session's entries in its list, as Paseo's stream ends:
//! the person's message the moment it is sent (Paseo's pending
//! `UserMessage`, its `clientMessageId` until the server's own record
//! comes), and the working footer while the agent works (`turn-footer.tsx`,
//! `RunningTurnFooter`).
//!
//! [Decision Log]
//! - Purpose: a message sent to an ended session sat in a box over the
//!   composer under "Restarting the session…", beside a card saying the
//!   session had ended (the owner's report on 0.0.0.0.13); a message sent
//!   to a live session showed only once the node's record came back. In
//!   Paseo the message is in the conversation at once and the agent's
//!   state is a quiet spinner at the end.
//! - Constraint: the list's rows are the timeline's entries (the node's
//!   records folded); a message not yet recorded has no entry.
//! - Choice: rows after the entries' rows, kept here: the pending message
//!   (its text, from the send until the node records a prompt with the
//!   same words, or the send fails and the text goes back to the
//!   composer), then the working footer while a turn runs or the session
//!   is being started again. They are spliced in and out of the list as
//!   they come and go, so the view stays where the person is.
//! - Cost: a pending message and the prompt that replaces it are matched
//!   by their words; two identical messages sent at once settle together.

use std::cell::{Cell, RefCell};
use std::time::Duration;

use chrono::{DateTime, Utc};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::timeline::EntryKind;

use super::ConversationArea;
use crate::shell::Shell;

/// How long after the working footer's elapsed time changes it is drawn
/// again: past the boundary, so the label read then is the new one.
const PAST: Duration = Duration::from_millis(20);

/// A row after the entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TailRow {
    /// The person's message, sent and not yet recorded.
    Pending(String),
    /// The agent works: since when, once its turn's prompt is in.
    Working(Option<DateTime<Utc>>),
}

/// The rows after the entries.
#[derive(Debug, Default)]
pub(crate) struct Tail {
    /// The message sent and not yet recorded.
    pending: Option<String>,
    /// When it was sent: it arrives on screen (`motion`).
    pending_at: Option<std::time::Instant>,
    /// Its first frame was logged (the time from Enter to the message
    /// drawn, slice M's measure).
    pending_logged: Cell<bool>,
    /// The rows the list holds now, after the entries' rows.
    shown: RefCell<Vec<TailRow>>,
    /// A redraw is due for the working footer's elapsed time.
    ticking: Cell<bool>,
}

#[cfg(test)]
impl Tail {
    /// The resident snapshot cannot discard this owner's live work/UI state.
    pub(super) fn snapshot_busy(&self) -> bool {
        self.pending.is_some() || self.ticking.get()
    }
}

impl ConversationArea {
    /// The rows wanted now: the pending message, then the working footer
    /// while `running` (a turn runs) or a message waits.
    fn tail_wanted(&self, running: bool) -> Vec<TailRow> {
        let mut rows = Vec::new();
        if let Some(text) = &self.tail.pending {
            rows.push(TailRow::Pending(text.clone()));
            // A message steered into (or interrupting) the running turn:
            // the turn's elapsed time goes on under it.
            rows.push(TailRow::Working(
                running.then(|| self.turn_started()).flatten(),
            ));
        } else if running {
            rows.push(TailRow::Working(self.turn_started()));
        }
        rows
    }

    /// When the turn running began: the last prompt's record.
    fn turn_started(&self) -> Option<DateTime<Utc>> {
        self.timeline
            .entries()
            .iter()
            .rev()
            .find(|entry| matches!(entry.kind, EntryKind::Prompt(_)))
            .map(|entry| entry.at)
    }

    /// The list's tail as wanted now: rows that came or went are spliced
    /// in or out after the entries' rows.
    pub(super) fn sync_tail(&self, running: bool) {
        let wanted = self.tail_wanted(running);
        let mut shown = self.tail.shown.borrow_mut();
        if *shown == wanted {
            return;
        }
        let start = self.at(self.rows.len());
        self.list
            .splice(start..start.saturating_add(shown.len()), wanted.len());
        *shown = wanted;
    }

    /// The pending message is drawn: the first time, how long after it
    /// was sent (debug log, for the measure).
    pub(super) fn pending_drawn(&self) {
        if self.tail.pending_logged.replace(true) {
            return;
        }
        if let Some(at) = self.tail.pending_at {
            tracing::debug!(after_us = at.elapsed().as_micros(), "a message sent: drawn");
        }
    }

    /// Whether the pending message was sent just now.
    pub(super) fn pending_fresh(&self) -> bool {
        self.tail
            .pending_at
            .is_some_and(|at| at.elapsed() < super::motion::FRESH)
    }

    /// The tail row at list row `row`, when it is one.
    pub(super) fn tail_row(&self, row: usize) -> Option<TailRow> {
        let at = row.checked_sub(self.rows.len())?;
        self.tail.shown.borrow().get(at).cloned()
    }

    /// The list was set to the entries' rows alone: the tail comes back
    /// at the next draw.
    pub(super) fn tail_dropped(&self) {
        self.tail.shown.borrow_mut().clear();
    }

    /// Entries from `from` came in: a prompt or a steer with the pending
    /// words settles it.
    pub(super) fn tail_took(&mut self, from: usize) {
        let Some(text) = self.tail.pending.as_deref() else {
            return;
        };
        let recorded = self
            .timeline
            .entries()
            .get(from..)
            .unwrap_or_default()
            .iter()
            .any(|entry| {
                // A message steered into the running turn is recorded as
                // the turn's steer, any other as a prompt.
                let sent = if let EntryKind::Prompt(prompt) = &entry.kind {
                    Some(prompt.text.as_str())
                } else if let EntryKind::Steer { text: steered, .. } = &entry.kind {
                    Some(steered.as_str())
                } else {
                    None
                };
                sent.is_some_and(|sent| sent.trim() == text.trim())
            });
        if recorded {
            self.tail.pending = None;
        }
    }
}

/// Keep the list's tail as wanted; while the agent works, draw again when
/// its elapsed time next reads differently.
///
/// [Decision Log]
/// - Purpose: the footer's time ("42s", "3m 5s", "2h 14m") was drawn again
///   every second by notifying the shell, which draws the whole window: a
///   session working for hours drew 60 frames a minute for one change, 20 %
///   of a core on a software renderer (track F's measure, 2026-11-14).
/// - Choice: wake when the label changes (`next_change`: the next second
///   under an hour, the next minute after) and draw only the timeline's
///   region (`crate::region`), not the shell's other parts.
/// - Cost: under an hour the label shows seconds, so it still draws once a
///   second, as Paseo's footer does.
pub(super) fn sync(shell: &Shell, window: &Window, cx: &mut Context<'_, Shell>) {
    let area = &shell.areas.conversation;
    let running = shell.areas.session.conversation.turn_running();
    area.sync_tail(running);
    let since = area.tail.shown.borrow().iter().find_map(|row| match row {
        TailRow::Working(Some(since)) => Some(*since),
        TailRow::Working(None) | TailRow::Pending(_) => None,
    });
    let Some(since) = since else {
        return;
    };
    if area.tail.ticking.replace(true) {
        return;
    }
    let wait = next_change(since, Utc::now()).saturating_add(PAST);
    cx.spawn_in(window, async move |this, cx| {
        cx.background_executor().timer(wait).await;
        let _gone = this.update(cx, |shell, cx| {
            shell.areas.conversation.tail.ticking.set(false);
            match shell.areas.regions.timeline.clone() {
                Some(region) => region.update(cx, |_region, cx| cx.notify()),
                None => cx.notify(),
            }
        });
    })
    .detach();
}

/// How long from `now` until the footer's elapsed time since `since`
/// reads differently (`footer::duration_label`): seconds are shown under
/// an hour, minutes after.
fn next_change(since: DateTime<Utc>, now: DateTime<Utc>) -> Duration {
    const HOUR_MS: i64 = 3_600_000;
    let elapsed = now.signed_duration_since(since).num_milliseconds().max(0);
    let step: i64 = if elapsed < HOUR_MS { 1_000 } else { 60_000 };
    let left = step.saturating_sub(elapsed.rem_euclid(step));
    Duration::from_millis(u64::try_from(left).unwrap_or(1_000))
}

impl Shell {
    /// A message was sent to the session shown and starts a turn (or the
    /// session again): it shows in the conversation at once.
    pub(crate) fn conversation_sending(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        if text.trim().is_empty() {
            return;
        }
        self.areas.conversation.tail.pending = Some(text.to_owned());
        self.areas.conversation.tail.pending_at = Some(std::time::Instant::now());
        self.areas.conversation.tail.pending_logged.set(false);
        tracing::debug!("a message sent: pending");
        cx.notify();
    }

    /// The message sent did not go (the link refused it, or the session
    /// could not be started again): it leaves the conversation; the
    /// composer has its words back.
    pub(crate) fn conversation_send_failed(&mut self) {
        self.areas.conversation.tail.pending = None;
    }
}

// Owner snapshot preparation; no staged exemption in the product path.
#[cfg(test)]
impl Tail {
    pub(super) fn snapshot_busy(&self) -> bool {
        self.pending.is_some() || self.ticking.get()
    }
}

#[cfg(test)]
mod tick_tests {
    use chrono::{TimeZone as _, Utc};

    use super::next_change;

    /// The footer is drawn again when its label changes: each second under
    /// an hour, each minute after (a session working for hours drew every
    /// second for a label that changed once a minute).
    #[test]
    fn the_footer_wakes_when_its_label_changes() {
        let since = Utc.with_ymd_and_hms(2026, 11, 14, 0, 0, 0).unwrap();
        let at = |ms: i64| since + chrono::Duration::milliseconds(ms);
        assert_eq!(next_change(since, at(1_250)).as_millis(), 750);
        assert_eq!(next_change(since, at(59_000)).as_millis(), 1_000);
        assert_eq!(next_change(since, at(3_600_000)).as_millis(), 60_000);
        assert_eq!(
            next_change(since, at(3_600_000 + 15_500)).as_millis(),
            44_500
        );
        assert_eq!(
            next_change(since, at(-5_000)).as_millis(),
            1_000,
            "a clock behind the node's"
        );
    }
}
