//! When a paired device's presence renews its Web Push lease (ADR 0089,
//! "Desktop idle evidence and lease renewal").
//!
//! [Decision Log]
//! - Purpose: a lease lasts 48 hours, and the store extends it only in its
//!   last 24 (`vessel_store::PUSH_LEASE`). A device that keeps reporting
//!   presence then keeps receiving pushes without subscribing again.
//! - Constraint: `push.presence` has been live since protocol 22 and is
//!   answered from memory. A report must not wait for the store's writer.
//!   A writer failure (closed while the node stops, a disk error) must not
//!   fail a presence the node has already recorded (the node reviewer's R1
//!   on TRACKER 815).
//! - Alternatives:
//!   - awaiting the renewal in the handler (the staging line): every report
//!     waited for a write that changes a row at most once a day, and a
//!     writer error failed the report;
//!   - gating the renewal behind `ATTENTION_NOTIFICATIONS_PROTOCOL`: the
//!     renewal is the node's own and not on the wire, so the gate would
//!     only delay leases that live through presence;
//!   - reading the lease's expiry on every report: a store read per report.
//! - Choice:
//!   - a device's renewal is tried at most once an hour, as a task on the
//!     node's tracker, and its failure is only logged;
//!   - a new subscription is not tried before its half-life;
//!   - an hour is well inside the 24 hours in which the store renews, so a
//!     device present at any time in that day is renewed.
//! - Cost: Paseo renews at the first heartbeat past the half-life; here it
//!   can be up to an hour later, which still leaves the lease a day.
//! - Must stay true: the presence reply never depends on this task.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use chrono::{DateTime, TimeDelta, Utc};
use vessel_proto::DeviceId;

/// How long after one try a device's presence tries again.
pub(super) const RETRY: TimeDelta = TimeDelta::hours(1);

/// Devices remembered at once. Past it, those whose time has passed are
/// forgotten; a forgotten device only tries again sooner.
const REMEMBERED: usize = 256;

/// The time before which each device's presence does not try to renew.
#[derive(Debug, Default)]
pub(super) struct Renewals(Mutex<HashMap<DeviceId, DateTime<Utc>>>);

impl Renewals {
    /// Whether `device` should try now. When it should, its next try is
    /// [`RETRY`] later.
    pub(super) fn due(&self, device: DeviceId, now: DateTime<Utc>) -> bool {
        let next = now.checked_add_signed(RETRY).unwrap_or(now);
        self.not_before(device, now, |at| now >= at, next)
    }

    /// A lease was just given until `expires_at`: the store will not renew
    /// it before its half-life, so its presence does not try before then.
    pub(super) fn leased(&self, device: DeviceId, expires_at: DateTime<Utc>, now: DateTime<Utc>) {
        let half = vessel_store::PUSH_LEASE
            .checked_div(2)
            .and_then(|half| expires_at.checked_sub_signed(half))
            .unwrap_or(now);
        let _set = self.not_before(device, now, |_| true, half);
    }

    /// Set `device`'s time to `next` when `replace` accepts its current one
    /// (or it has none), keeping at most [`REMEMBERED`] devices.
    fn not_before(
        &self,
        device: DeviceId,
        now: DateTime<Utc>,
        replace: impl FnOnce(DateTime<Utc>) -> bool,
        next: DateTime<Utc>,
    ) -> bool {
        let mut times = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(at) = times.get(&device).copied() {
            if !replace(at) {
                return false;
            }
        } else if times.len() >= REMEMBERED {
            times.retain(|_, at| now < *at);
            if times.len() >= REMEMBERED {
                times.clear();
            }
        }
        let _previous = times.insert(device, next);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hours: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_776_600_000, 0)
            .unwrap()
            .checked_add_signed(TimeDelta::hours(hours))
            .unwrap()
    }

    #[test]
    fn a_device_tries_once_an_hour() {
        let renewals = Renewals::default();
        let device = DeviceId::random();
        assert!(renewals.due(device, at(0)));
        assert!(!renewals.due(device, at(0)));
        assert!(
            renewals.due(DeviceId::random(), at(0)),
            "another device is its own"
        );
        let almost = at(1).checked_sub_signed(TimeDelta::seconds(1)).unwrap();
        assert!(!renewals.due(device, almost));
        assert!(renewals.due(device, at(1)));
    }

    #[test]
    fn a_new_lease_is_not_tried_before_its_half_life() {
        let renewals = Renewals::default();
        let device = DeviceId::random();
        renewals.leased(device, at(48), at(0));
        assert!(!renewals.due(device, at(23)));
        assert!(renewals.due(device, at(24)));
        // A new subscription moves the time on again, even after a try.
        renewals.leased(device, at(73), at(25));
        assert!(!renewals.due(device, at(48)));
        assert!(renewals.due(device, at(49)));
    }

    #[test]
    fn a_full_table_forgets_passed_devices_first() {
        let renewals = Renewals::default();
        let kept = DeviceId::random();
        renewals.leased(kept, at(48), at(0));
        for _ in 1..REMEMBERED {
            assert!(renewals.due(DeviceId::random(), at(0)));
        }
        // At two hours every other device's time has passed: they are
        // forgotten to make room, and the lease's device keeps its time.
        assert!(renewals.due(DeviceId::random(), at(2)));
        assert!(!renewals.due(kept, at(2)));
    }
}
