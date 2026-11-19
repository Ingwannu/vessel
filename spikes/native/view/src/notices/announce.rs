//! Which notices are told, and how (ADR 0114): Paseo's attention rules on
//! top of the diff in [`super::notices`].
//!
//! [Decision Log]
//! - Purpose: the system's notification and the sound go to a person who
//!   is not already looking at the session, as Paseo's
//!   `notifyAgentAttention` (`session-context.tsx`) decides.
//! - Before: the page told a notice only while it was away (hidden or not
//!   focused), and native told nothing.
//! - Choice:
//!   - Paseo's rule: a notice is dropped only when the app is in front and
//!     shows that session. In front on another session, it is told.
//!   - How it is told is Settings' choice (`Alerts`): the system's
//!     notification, the sound, both, or nothing.
//!   - The title counts the notices since the app was last in front,
//!     whatever the choice, as the page's tab title does.
//! - Not here: one recipient among several open apps. That is the node's
//!   claim (ADR 0089, TRACKER 815). Until it lands, each open app decides
//!   for itself (a stated difference).

use vessel_proto::SessionId;

use super::Notice;
use crate::prefs::Alerts;

/// Where the person is looking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen<'a> {
    /// The app is in front: its window is the active one (on the page,
    /// visible and focused).
    pub active: bool,
    /// The conversation it shows, by machine and session.
    pub shown: Option<(&'a str, SessionId)>,
}

impl Seen<'_> {
    /// Whether the person is looking at `notice`'s session now.
    fn looking_at(&self, notice: &Notice) -> bool {
        self.active
            && self.shown.is_some_and(|(machine, session)| {
                machine == notice.machine && session == notice.session
            })
    }
}

/// One notice to tell, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announce {
    /// What.
    pub notice: Notice,
    /// As the system's notification.
    pub system: bool,
    /// With the sound.
    pub sound: bool,
}

/// The notices to tell, in order, under Settings' `alerts`.
#[must_use]
pub fn announce(found: Vec<Notice>, seen: Seen<'_>, alerts: Alerts) -> Vec<Announce> {
    let (system, sound) = (alerts.system(), alerts.sound());
    if !system && !sound {
        return Vec::new();
    }
    found
        .into_iter()
        .filter(|notice| !seen.looking_at(notice))
        .map(|notice| Announce {
            notice,
            system,
            sound,
        })
        .collect()
}

/// The notices the title counts: those since the app was last in front.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Unseen(usize);

impl Unseen {
    /// Count `found` notices, if the app is not in front and the title
    /// counts. True when the count changed.
    pub fn found(&mut self, found: usize, active: bool, counts: bool) -> bool {
        if active || !counts || found == 0 {
            return false;
        }
        self.0 = self.0.saturating_add(found);
        true
    }

    /// The app came to the front. True when there was a count to clear.
    pub fn back(&mut self) -> bool {
        std::mem::take(&mut self.0) > 0
    }

    /// How many.
    #[must_use]
    pub const fn count(self) -> usize {
        self.0
    }
}
