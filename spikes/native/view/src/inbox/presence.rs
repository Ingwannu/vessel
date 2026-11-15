//! Client presence is driven by actual input, focus and connection transitions.
//! Interval ticks preserve activity age; they must never make an idle socket
//! look like a person using the app.
use std::time::{Duration, Instant};
use vessel_proto::{NodeId, SessionId, TerminalId};

/// Paseo's ordinary heartbeat interval.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
/// Paseo's activity-triggered heartbeat throttle.
pub const ACTIVITY_THROTTLE: Duration = Duration::from_secs(5);

/// Aggregate OS idle duration, never input text or pointer coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleAge(u32);
impl IdleAge {
    /// Milliseconds reported by the platform's aggregate idle counter.
    #[must_use]
    pub const fn new(milliseconds: u32) -> Self {
        Self(milliseconds)
    }
    /// Milliseconds since the last system input.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
/// A sample keeps aging in a queue: repeating a cached short age must never
/// make an abandoned connection look active indefinitely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleSample {
    age: IdleAge,
    observed: Instant,
}
impl IdleSample {
    /// Bind a platform reading to the app's monotonic clock.
    #[must_use]
    pub const fn new(age: IdleAge, observed: Instant) -> Self {
        Self { age, observed }
    }
    /// Saturate at the wire maximum rather than wrapping back to active.
    #[must_use]
    pub fn age_at(self, now: Instant) -> IdleAge {
        let elapsed = u32::try_from(now.saturating_duration_since(self.observed).as_millis())
            .unwrap_or(u32::MAX);
        IdleAge::new(self.age.get().saturating_add(elapsed))
    }
}

/// Typed navigation focus; terminal support is refused until its node
/// contract exists rather than silently treating it as a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// A visible conversation.
    Session {
        /// Reporting node owning the conversation.
        node: NodeId,
        /// Visible session on that node.
        session: SessionId,
    },
    /// A visible terminal.
    Terminal {
        /// Reporting node owning the terminal.
        node: NodeId,
        /// Visible terminal on that node.
        terminal: TerminalId,
    },
    /// A visible screen with no focused attention source.
    Elsewhere,
    /// Hidden, minimized or unfocused.
    Background,
}

/// The node distinguishes real input from visibility-only refreshes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// Input occurred since the last report on this connection.
    Active,
    /// Preserve the node's previous activity timestamp.
    Unchanged,
}

/// One coalesced report, without input text, pointer coordinates or timestamps
/// supplied by the client to extend the node's presence into the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Current navigation/visibility.
    pub focus: Focus,
    /// Whether real input renews the node-assigned activity timestamp.
    pub activity: Activity,
    /// Optional OS sample; absence means unknown, never zero idle.
    pub idle: Option<IdleSample>,
}

/// Pure scheduler state, with a clock injected by the app boundary.
#[derive(Debug)]
pub struct Presence {
    focus: Focus,
    idle: Option<IdleSample>,
    pending: Activity,
    sent_at: Option<Instant>,
    immediate_at: Option<Instant>,
}
impl Default for Presence {
    fn default() -> Self {
        Self {
            focus: Focus::Background,
            idle: None,
            pending: Activity::Unchanged,
            sent_at: None,
            immediate_at: None,
        }
    }
}
impl Presence {
    /// Current focus for a new connection, without replaying offline input.
    #[must_use]
    pub const fn snapshot(&self) -> Report {
        Report {
            focus: self.focus,
            activity: Activity::Unchanged,
            idle: None,
        }
    }
    /// A focus/visibility transition sends immediately but does not invent input.
    #[must_use]
    pub fn focus(&mut self, focus: Focus, now: Instant) -> Option<Report> {
        if self.focus == focus {
            return None;
        }
        self.focus = focus;
        Some(self.report(now))
    }
    /// Actual keyboard, pointer or touch activity is coalesced at five seconds.
    #[must_use]
    pub fn input(&mut self, now: Instant) -> Option<Report> {
        self.pending = Activity::Active;
        if self
            .immediate_at
            .is_some_and(|at| now.saturating_duration_since(at) < ACTIVITY_THROTTLE)
        {
            return None;
        }
        self.immediate_at = Some(now);
        Some(self.report(now))
    }
    /// The regular timer sends only due reports and never renews activity by
    /// itself. Clock regressions saturate rather than underflowing.
    #[must_use]
    pub fn tick(&mut self, now: Instant) -> Option<Report> {
        if self
            .sent_at
            .is_some_and(|at| now.saturating_duration_since(at) < HEARTBEAT_INTERVAL)
        {
            return None;
        }
        let mut report = self.report(now);
        if report.activity == Activity::Unchanged {
            report.idle = self.idle;
        }
        Some(report)
    }
    /// Cache bounded OS evidence without creating a heartbeat or postponing
    /// its deadline. The next ordinary report ages the original sample.
    pub fn system_idle(&mut self, sample: Option<IdleSample>) {
        self.idle = sample;
    }
    /// A new connection sends current focus without replaying stale offline
    /// input into a fresh node-assigned activity timestamp.
    #[must_use]
    pub fn reconnect(&mut self, now: Instant) -> Report {
        self.pending = Activity::Unchanged;
        self.immediate_at = None;
        self.report(now)
    }
    fn report(&mut self, now: Instant) -> Report {
        self.sent_at = Some(now);
        let activity = std::mem::replace(&mut self.pending, Activity::Unchanged);
        Report {
            focus: self.focus,
            activity,
            idle: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_ticks_do_not_renew_activity_and_input_is_throttled_without_being_lost() {
        let now = Instant::now();
        let mut presence = Presence::default();
        assert_eq!(presence.tick(now).unwrap().activity, Activity::Unchanged);
        assert_eq!(presence.input(now).unwrap().activity, Activity::Active);
        assert_eq!(presence.input(now), None);
        assert_eq!(presence.tick(now), None);
        let later = now.checked_add(HEARTBEAT_INTERVAL).unwrap();
        assert_eq!(presence.tick(later).unwrap().activity, Activity::Active);
        let idle = later.checked_add(HEARTBEAT_INTERVAL).unwrap();
        assert_eq!(presence.tick(idle).unwrap().activity, Activity::Unchanged);
    }
    #[test]
    fn focus_and_reconnect_are_immediate_but_never_synthesize_a_persons_input() {
        let now = Instant::now();
        let mut presence = Presence::default();
        let session = SessionId::random();
        let node = NodeId::random();
        assert_eq!(
            presence.focus(Focus::Session { node, session }, now),
            Some(Report {
                focus: Focus::Session { node, session },
                activity: Activity::Unchanged,
                idle: None
            })
        );
        assert_eq!(presence.focus(Focus::Session { node, session }, now), None);
        assert_eq!(presence.input(now).unwrap().activity, Activity::Active);
        assert_eq!(presence.input(now), None);
        assert_eq!(presence.reconnect(now).activity, Activity::Unchanged);
        assert_eq!(presence.tick(now), None);
    }
    #[test]
    fn cached_os_idle_ages_and_unknown_never_becomes_zero_idle() {
        let now = Instant::now();
        let mut presence = Presence::default();
        presence.system_idle(Some(IdleSample::new(IdleAge::new(100_000), now)));
        let report = presence.tick(now).unwrap();
        assert_eq!(report.activity, Activity::Unchanged);
        let later = now.checked_add(Duration::from_secs(80)).unwrap();
        assert_eq!(report.idle.unwrap().age_at(later).get(), 180_000);
        let report = presence.tick(later).unwrap();
        assert_eq!(report.idle.unwrap().age_at(later).get(), 180_000);
        assert!(presence.input(later).unwrap().idle.is_none());
        assert!(presence.snapshot().idle.is_none());
        presence.system_idle(None);
        let after = later.checked_add(HEARTBEAT_INTERVAL).unwrap();
        assert!(presence.tick(after).unwrap().idle.is_none());
    }
}

#[cfg(test)]
mod idle_schedule_tests {
    use super::*;
    #[test]
    fn idle_sampling_cannot_turn_fifteen_second_heartbeats_into_probe_frequency() {
        let now = Instant::now();
        let mut presence = Presence::default();
        assert!(presence.tick(now).is_some());
        for delay in [ACTIVITY_THROTTLE, HEARTBEAT_INTERVAL - ACTIVITY_THROTTLE] {
            let sample_at = now.checked_add(delay).unwrap();
            presence.system_idle(Some(IdleSample::new(IdleAge::new(0), sample_at)));
            assert!(presence.tick(sample_at).is_none());
        }
        let due = now.checked_add(HEARTBEAT_INTERVAL).unwrap();
        let report = presence.tick(due).unwrap();
        assert_eq!(report.activity, Activity::Unchanged);
        assert!(report.idle.is_some());
        assert!(presence.tick(due).is_none());
    }
}
