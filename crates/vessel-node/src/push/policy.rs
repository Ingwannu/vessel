//! Paseo's attention policy as pure rules, with its 180-second fixtures.
//! Whole-window silence for desktop sound is a separate client preference.
use chrono::{DateTime, TimeDelta, Utc};
use vessel_proto::SessionId;
use vessel_proto::push::{PushActivity, PushFocus, ReportPushPresence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Presence {
    pub(crate) focus: PushFocus,
    pub(crate) active_at: Option<DateTime<Utc>>,
    /// The window tells notices as the system's notifications (Paseo's
    /// `notifications: true`); only such a window is elected.
    pub(crate) notifies: bool,
}
impl Presence {
    pub(crate) fn report(&mut self, report: ReportPushPresence, now: DateTime<Utc>) {
        self.focus = report.focus;
        self.notifies = report.notifies;
        match report.activity {
            PushActivity::Active => self.active_at = Some(now),
            PushActivity::Unchanged => {}
        }
        if let Some(age) = report.idle_age {
            let observed = now.checked_sub_signed(TimeDelta::milliseconds(i64::from(age.get())));
            self.active_at = self.active_at.max(observed);
        }
    }
    pub(crate) fn present(self, now: DateTime<Utc>) -> bool {
        self.active_at
            .is_some_and(|at| now.signed_duration_since(at.min(now)) <= TimeDelta::seconds(180))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Plan {
    Suppressed,
    Local(usize),
    Push,
}
pub(crate) fn plan(states: &[Presence], session: SessionId, now: DateTime<Utc>) -> Plan {
    plan_for(states, PushFocus::Session { session }, now)
}
/// The local recipient is elected only among present windows that notify,
/// as Paseo elects among clients subscribed with `notifications: true`
/// (`websocket-server.ts:2883-2886,2987-2990`).
///
/// A deliberate difference (ADR 0089, "Turned on", item 5): every present
/// window still counts for suppression and for push. A focused one that
/// does not notify suppresses, and a person present only at such windows is
/// not pushed; no window is admitted, so each plays its own sound (ADR
/// 0114). Paseo builds its whole plan from the notifying clients only
/// (`allStates = notificationEntries`, then
/// `agent-attention-policy.ts:42-76`), so there a non-notifying client
/// neither suppresses nor holds push back. Its only such clients are not
/// its app, which always subscribes with `notifications: true`
/// (`session-context.tsx:492`); Vessel's are windows with a person at them.
pub(crate) fn plan_for(states: &[Presence], target: PushFocus, now: DateTime<Utc>) -> Plan {
    let mut recipient = None;
    let mut present = false;
    for (index, state) in states.iter().enumerate() {
        if !state.present(now) {
            continue;
        }
        present = true;
        if state.focus == target {
            return Plan::Suppressed;
        }
        if !state.notifies {
            continue;
        }
        if recipient.is_none_or(|previous: usize| {
            states.get(previous).is_some_and(|prior| {
                state.active_at.map(|at| at.min(now)) > prior.active_at.map(|at| at.min(now))
            })
        }) {
            recipient = Some(index);
        }
    }
    match recipient {
        Some(index) => Plan::Local(index),
        None if present => Plan::Suppressed,
        None => Plan::Push,
    }
}
