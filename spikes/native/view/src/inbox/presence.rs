//! Client presence is driven by actual input, focus and connection transitions.
//! Interval ticks preserve activity age; they must never make an idle socket
//! look like a person using the app.
use std::time::{Duration, Instant};
use vessel_proto::{NodeId, SessionId, TerminalId};

/// Paseo's ordinary heartbeat interval.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
/// Paseo's activity-triggered heartbeat throttle.
pub const ACTIVITY_THROTTLE: Duration = Duration::from_secs(5);

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
}

/// Pure scheduler state, with a clock injected by the app boundary.
#[derive(Debug)]
pub struct Presence {
    focus: Focus,
    pending: Activity,
    sent_at: Option<Instant>,
    immediate_at: Option<Instant>,
}
impl Default for Presence {
    fn default() -> Self {
        Self {
            focus: Focus::Background,
            pending: Activity::Unchanged,
            sent_at: None,
            immediate_at: None,
        }
    }
}
impl Presence {
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
        Some(self.report(now))
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
                activity: Activity::Unchanged
            })
        );
        assert_eq!(presence.focus(Focus::Session { node, session }, now), None);
        assert_eq!(presence.input(now).unwrap().activity, Activity::Active);
        assert_eq!(presence.input(now), None);
        assert_eq!(presence.reconnect(now).activity, Activity::Unchanged);
        assert_eq!(presence.tick(now), None);
    }
}
