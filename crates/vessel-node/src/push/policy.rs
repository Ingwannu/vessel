//! Paseo's attention policy as pure rules, with its 180-second fixtures.
//! Whole-window silence for desktop sound is a separate client preference.
use chrono::{DateTime, TimeDelta, Utc};
use vessel_proto::SessionId;
use vessel_proto::push::{PushActivity, PushFocus, ReportPushPresence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Presence {
    pub(crate) focus: PushFocus,
    pub(crate) active_at: Option<DateTime<Utc>>,
}
impl Presence {
    pub(crate) fn report(&mut self, report: ReportPushPresence, now: DateTime<Utc>) {
        self.focus = report.focus;
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
pub(crate) fn plan_for(states: &[Presence], target: PushFocus, now: DateTime<Utc>) -> Plan {
    let mut recipient = None;
    for (index, state) in states.iter().enumerate() {
        if !state.present(now) {
            continue;
        }
        if state.focus == target {
            return Plan::Suppressed;
        }
        if recipient.is_none_or(|previous: usize| {
            states.get(previous).is_some_and(|prior| {
                state.active_at.map(|at| at.min(now)) > prior.active_at.map(|at| at.min(now))
            })
        }) {
            recipient = Some(index);
        }
    }
    recipient.map_or(Plan::Push, Plan::Local)
}
