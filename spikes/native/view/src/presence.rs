//! The window's presence on each machine (`push.presence`, protocol 22;
//! ADR 0041, ADR 0089): whether a person is here, and what they look at,
//! so a machine sends no Web Push to someone in front of the app.
//!
//! [Decision Log]
//! - Purpose: a machine already decides from presence whether to push
//!   (Paseo's policy, vessel-node `push/policy.rs`): push only when no
//!   window reported real input in the last 180 s, and nothing at all when
//!   a visible window shows that very session. The app never reported
//!   presence, so every push went out while the person used the app (ADR
//!   0114's stated difference).
//! - Reference: Paseo 0.15.4, `packages/app/src/hooks` in
//!   ~/refs/paseo-latest:
//!   - `client-activity-tracker.ts:1-2`: a heartbeat every 15 s, immediate
//!     ones at most every 5 s ([`HEARTBEAT`], [`THROTTLE`]);
//!   - `:43`: opening counts as input;
//!   - `:63-73`: input records the time, then tries an immediate heartbeat
//!     ([`Presence::input`], [`Presence::immediate`]);
//!   - `:78-89`: a new shown session is input and sends at once
//!     ([`Presence::show`]);
//!   - `:90-105`: becoming visible is input; becoming hidden sends nothing
//!     until the next heartbeat ([`Presence::visibility`]);
//!   - `use-client-activity.ts:125-148`: a heartbeat on connecting, then
//!     every 15 s while connected;
//!   - the page does the same (`web/src/data/presence.ts`).
//! - Visible is the window's visibility, as Paseo's desktop reads the
//!   document's: shown on screen, active or not. A minimized or hidden
//!   window reports `Background`; a visible one that is not active still
//!   shows its session, as in Paseo.
//! - Difference, by the protocol: Paseo sends the input's own time; Vessel
//!   sends `Active` and the machine stamps its own time (ADR 0089: no
//!   client wall clocks). Presence so ends 180 s after the report carrying
//!   the input, at most one heartbeat (15 s) after the input. A machine
//!   that connects hears `Active` only for input in the last 15 s, the same
//!   bound.
//! - Choice: this module is pure, with the clock passed in as milliseconds;
//!   the app records input and visibility, and the link sends each
//!   machine its report through [`Told`].
//! - A shown terminal (claims row 3): Paseo's focus target is an agent or
//!   a terminal (`agent-attention-policy.ts:12`). A focused terminal tab is
//!   reported as `Terminal` focus, so the machine suppresses that
//!   terminal's notice and keeps the session's. A report carrying
//!   `Terminal` focus is versioned to ADR 0089's constant
//!   (`rpc/since.rs`), so only a machine that admits claims hears it. Any
//!   other machine hears `Elsewhere`: the person is not looking at the
//!   session's conversation. This changes push today. Before, a terminal
//!   tab reported its route's session, and that session's push was held
//!   back; now it goes, as Paseo's does while a terminal has the focus.

use std::collections::HashMap;
use std::hash::Hash;
use std::time::Duration;

use vessel_proto::push::{PushActivity, PushFocus, ReportPushPresence};
use vessel_proto::{SessionId, TerminalId};

/// What the window shows on a machine: a session's conversation, or a
/// terminal's tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// A session's conversation.
    Session(SessionId),
    /// A terminal's tab.
    Terminal(TerminalId),
}

impl From<SessionId> for Shown {
    fn from(session: SessionId) -> Self {
        Self::Session(session)
    }
}

/// Paseo's `HEARTBEAT_INTERVAL_MS` (`client-activity-tracker.ts:1`).
pub const HEARTBEAT: Duration = Duration::from_secs(15);

/// Paseo's `ACTIVITY_HEARTBEAT_THROTTLE_MS` (`client-activity-tracker.ts:2`).
pub const THROTTLE: Duration = Duration::from_secs(5);

/// Milliseconds on the app's own monotonic clock.
pub type Millis = u64;

/// `duration` in milliseconds, saturating.
fn millis(duration: Duration) -> Millis {
    Millis::try_from(duration.as_millis()).unwrap_or(Millis::MAX)
}

/// What the window is doing, as Paseo's tracker keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    last_input: Millis,
    visible: bool,
    shown: Option<(String, Shown)>,
    last_immediate: Option<Millis>,
}

impl Presence {
    /// Opening the window is input (Paseo's `:43`).
    #[must_use]
    pub const fn new(now: Millis, visible: bool) -> Self {
        Self {
            last_input: now,
            visible,
            shown: None,
            last_immediate: None,
        }
    }

    /// Real input: a key, a pointer, a wheel, the window activated.
    pub const fn input(&mut self, now: Millis) {
        self.last_input = now;
    }

    /// Whether an immediate heartbeat may go now: not within 5 s of the
    /// last one.
    pub fn immediate(&mut self, now: Millis) -> bool {
        let due = self
            .last_immediate
            .is_none_or(|last| now.saturating_sub(last) >= millis(THROTTLE));
        if due {
            self.last_immediate = Some(now);
        }
        due
    }

    /// The shown session or terminal, on its machine. A change is input and
    /// sends at once (`true`); the same one sends nothing.
    pub fn show(&mut self, shown: Option<(&str, Shown)>, now: Millis) -> bool {
        let same = match (&self.shown, shown) {
            (Some((machine, was)), Some((next_machine, next))) => {
                machine == next_machine && *was == next
            }
            (None, None) => true,
            _ => false,
        };
        if same {
            return false;
        }
        self.shown = shown.map(|(machine, what)| (machine.to_owned(), what));
        self.input(now);
        true
    }

    /// The window's visibility; whether it changed. Becoming visible is
    /// input.
    pub const fn visibility(&mut self, visible: bool, now: Millis) -> bool {
        if self.visible == visible {
            return false;
        }
        self.visible = visible;
        if visible {
            self.input(now);
        }
        true
    }

    /// Whether the window is visible.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }

    /// What the link needs to report to each machine at `now`.
    #[must_use]
    pub fn snapshot(&self, now: Millis) -> Snapshot {
        Snapshot {
            shown: self.shown.clone(),
            visible: self.visible,
            last_input: self.last_input,
            now,
        }
    }
}

/// The window's presence at one moment, handed to the link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    shown: Option<(String, Shown)>,
    visible: bool,
    last_input: Millis,
    now: Millis,
}

impl Snapshot {
    /// What `machine` is shown: its session, or its terminal when
    /// `terminals` (the machine admits claims), when the window is visible
    /// and shows one; elsewhere when visible; background when hidden.
    #[must_use]
    pub fn focus(&self, machine: &str, terminals: bool) -> PushFocus {
        if !self.visible {
            return PushFocus::Background;
        }
        match &self.shown {
            Some((on, Shown::Session(session))) if on == machine => {
                PushFocus::Session { session: *session }
            }
            Some((on, Shown::Terminal(terminal))) if on == machine && terminals => {
                PushFocus::Terminal {
                    terminal: *terminal,
                }
            }
            _ => PushFocus::Elsewhere,
        }
    }
}

/// What each machine connection was last told of input, so each report
/// says `Active` only for input it has not heard of.
#[derive(Debug)]
pub struct Told<K> {
    told: HashMap<K, Millis>,
}

impl<K> Default for Told<K> {
    fn default() -> Self {
        Self {
            told: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash> Told<K> {
    /// The report for `machine` on `connection`, `later` milliseconds after
    /// `snapshot` was taken. A connection not told before hears `Active`
    /// only for input in the last 15 s. `terminals`: the machine admits
    /// claims, so it may hear a shown terminal ([`Snapshot::focus`]).
    pub fn report(
        &mut self,
        connection: K,
        machine: &str,
        snapshot: &Snapshot,
        later: Millis,
        terminals: bool,
    ) -> ReportPushPresence {
        let before = self.told.insert(connection, snapshot.last_input);
        let fresh = before.map_or_else(
            || {
                let age = snapshot
                    .now
                    .saturating_sub(snapshot.last_input)
                    .saturating_add(later);
                age <= millis(HEARTBEAT)
            },
            |before| snapshot.last_input > before,
        );
        ReportPushPresence {
            focus: snapshot.focus(machine, terminals),
            activity: if fresh {
                PushActivity::Active
            } else {
                PushActivity::Unchanged
            },
            idle_age: None,
            // The meaning before the field (ADR 0089): this window may be a
            // claim's local recipient. U's native half decides it per window.
            notifies: true,
        }
    }

    /// Keep only the connections `current` still has.
    pub fn retain(&mut self, current: impl Fn(&K) -> bool) {
        self.told.retain(|connection, _told| current(connection));
    }
}

#[cfg(test)]
mod tests;
