//! How often one terminal's attention may become a Web Push (ADR 0089,
//! "Terminal Web Push joins the one worker"; owed item 1 of the node review
//! of TRACKER 815).
//!
//! [Decision Log]
//! - Purpose: an agent may report its own terminal's activity, and a hook can
//!   loop Working/Idle. Each Idle is a new Finished revision, so with no
//!   client present the worker would push once per 250 ms burst, about four
//!   a second per device. A service that crashes about every minute would
//!   push "finished" for as long as it runs.
//! - Constraint: the first attention after a quiet spell must still push at
//!   once, as Paseo's does, and one burst must reach every device: the rule
//!   is decided once per burst, not per delivery.
//! - Alternatives:
//!   - no limit (Paseo `websocket-server.ts` `broadcastTerminalAttention`
//!     pushes every transition): the flood above;
//!   - one fixed gap: a loop slower than the gap pushes forever at its own
//!     pace;
//!   - a limit per device or per node: one noisy terminal would silence the
//!     others.
//! - Choice: per terminal, a push opens a gap of one minute. A push at the
//!   end of a gap doubles it, up to an hour; a terminal quiet for twice its
//!   gap is back to one minute. Attention inside a gap is not pushed later:
//!   it stays on the terminal for clients to show, and Web Push is only an
//!   opportunity (ADR 0041).
//! - Effect: a turn that finishes every few minutes pushes each time; a loop
//!   or a crash loop pushes at 0, 1, 3, 7, 15, 31 and 63 minutes, then
//!   hourly.

use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use vessel_proto::TerminalId;

/// The gap after a push from a quiet terminal.
pub(super) const FIRST: TimeDelta = TimeDelta::minutes(1);

/// The longest gap.
pub(super) const LONGEST: TimeDelta = TimeDelta::hours(1);

/// One terminal's last push and the gap it opened.
#[derive(Debug, Clone, Copy)]
struct Last {
    at: DateTime<Utc>,
    gap: TimeDelta,
}

/// The pushes of the terminals the registry still holds.
#[derive(Debug, Default)]
pub(super) struct Spacing(HashMap<TerminalId, Last>);

impl Spacing {
    /// Whether `terminal`'s attention may be pushed at `now`.
    pub(super) fn allows(&self, terminal: TerminalId, now: DateTime<Utc>) -> bool {
        self.0.get(&terminal).is_none_or(|last| {
            last.at
                .checked_add_signed(last.gap)
                .is_none_or(|open| now >= open)
        })
    }

    /// One burst pushed `terminal`'s attention at `now`.
    pub(super) fn pushed(&mut self, terminal: TerminalId, now: DateTime<Utc>) {
        let gap = self.0.get(&terminal).map_or(FIRST, |last| {
            let quiet = last
                .gap
                .checked_mul(2)
                .and_then(|twice| last.at.checked_add_signed(twice))
                .is_none_or(|calm| now >= calm);
            if quiet {
                FIRST
            } else {
                last.gap
                    .checked_mul(2)
                    .map_or(LONGEST, |gap| gap.min(LONGEST))
            }
        });
        let _previous = self.0.insert(terminal, Last { at: now, gap });
    }

    /// Forget the terminals `kept` no longer holds.
    pub(super) fn retain(&mut self, kept: impl Fn(TerminalId) -> bool) {
        self.0.retain(|terminal, _last| kept(*terminal));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_776_600_000, 0)
            .unwrap()
            .checked_add_signed(TimeDelta::seconds(seconds))
            .unwrap()
    }

    /// What a terminal that asks every `every` seconds pushes within
    /// `until` seconds.
    fn pushes(every: i64, until: i64) -> Vec<i64> {
        let mut spacing = Spacing::default();
        let terminal = TerminalId::random();
        let mut sent = Vec::new();
        let mut now = 0;
        while now <= until {
            if spacing.allows(terminal, at(now)) {
                spacing.pushed(terminal, at(now));
                sent.push(now);
            }
            now = now.saturating_add(every);
        }
        sent
    }

    #[test]
    fn a_loop_backs_off_to_hourly() {
        let minutes: Vec<i64> = pushes(1, 14_400)
            .into_iter()
            .map(|second| second.checked_div(60).unwrap())
            .collect();
        assert_eq!(minutes, vec![0, 1, 3, 7, 15, 31, 63, 123, 183]);
    }

    #[test]
    fn a_crash_every_61_seconds_backs_off_too() {
        let sent = pushes(61, 10_800);
        assert_eq!(sent, vec![0, 61, 183, 427, 915, 1891, 3843, 7503]);
        let gaps: Vec<i64> = sent
            .windows(2)
            .map(|pair| pair[1].saturating_sub(pair[0]))
            .collect();
        assert!(gaps.last().is_some_and(|gap| *gap >= 3600), "{gaps:?}");
    }

    #[test]
    fn turns_minutes_apart_push_each_time() {
        assert_eq!(pushes(300, 1800), vec![0, 300, 600, 900, 1200, 1500, 1800]);
    }

    #[test]
    fn terminals_are_spaced_apart_from_each_other() {
        let mut spacing = Spacing::default();
        let one = TerminalId::random();
        let two = TerminalId::random();
        spacing.pushed(one, at(0));
        assert!(!spacing.allows(one, at(30)));
        assert!(spacing.allows(two, at(30)));
        spacing.retain(|terminal| terminal == two);
        assert!(
            spacing.allows(one, at(30)),
            "a forgotten terminal starts again"
        );
    }
}
