//! Settings > Devices (the page's `data/devices.ts`): a machine's paired
//! devices, and the pairing link a new one is added with.
//!
//! [Decision Log]
//! - Purpose: a pairing link is a credential for as long as it lives (ten
//!   minutes): whoever reads it can pair a device. The page shows it only
//!   when asked and hides it again after 30 seconds, when its window loses
//!   the eye, and once it is copied.
//! - Choice: the same rule as a value ([`LinkShown`]) the screen asks
//!   before drawing the link or its code; the link itself stays a
//!   [`vessel_proto::PairingLink`], whose `Debug` never prints it, so it
//!   reaches no log, no capture's name and no test's failure message.

mod details;
mod row;
pub use row::{Access, DeviceBadge};
mod invitation;
mod page;
pub mod qr;
mod target;
pub mod time;

pub use details::{DeviceState, details, exact};
pub use invitation::{DisplayLimit, Pairing, invitation_route};
pub use page::{Inviting, Listing, Problem, Revoking, Ticket, Tickets};
pub use target::{MachineKey, Page, Target};

use chrono::{DateTime, Duration, Utc};
use vessel_proto::DeviceInfo;

use crate::Lang;
use crate::ago::Ago;

/// How long a new pairing link works, in seconds (the page's).
pub const LINK_TTL_SECS: u32 = 600;

/// How long a shown pairing link stays shown, in seconds.
pub const LINK_SHOWN_SECS: i64 = 30;

/// The devices that still have access (a revoked one is not listed).
#[must_use]
pub fn paired_devices(devices: &[DeviceInfo]) -> Vec<&DeviceInfo> {
    devices
        .iter()
        .filter(|device| device.revoked_at.is_none())
        .collect()
}

/// A new device's label as the node is sent it: trimmed, and none when
/// nothing is left.
#[must_use]
pub fn invite_label(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// A device's row: what it may do, when it was last seen, when it was
/// paired.
#[must_use]
pub fn device_line(lang: Lang, device: &DeviceInfo, now: DateTime<Utc>) -> Vec<String> {
    let words = lang.machines();
    vec![
        scope_line(lang, device),
        device.last_seen.map_or_else(
            || words.never_seen.to_owned(),
            |seen| words.seen(Ago::of(seen, now)),
        ),
        words.paired(Ago::of(device.created_at, now)),
    ]
}

/// The legacy web-fixture scope sentence; native rows use a compact access
/// summary and keep their complete grants in the exact tooltip.
#[must_use]
pub fn scope_line(lang: Lang, device: &DeviceInfo) -> String {
    let words = lang.machines();
    let may: Vec<_> = device
        .scopes
        .iter()
        .map(|scope| words.scope(*scope))
        .collect();
    words.may(&may)
}

/// Whether a pairing link is shown, and since when.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LinkShown {
    /// Hidden: the screen says a link exists and offers to show it.
    #[default]
    Hidden,
    /// Shown since then.
    Since(DateTime<Utc>),
}

impl LinkShown {
    /// Whether to draw the link and its code at `now` (30 seconds after it
    /// was shown it is hidden again).
    #[must_use]
    pub fn visible(self, now: DateTime<Utc>) -> bool {
        match self {
            Self::Hidden => false,
            Self::Since(at) => now >= at && self.hides_at().is_some_and(|hides| now < hides),
        }
    }

    /// When it hides by itself, if shown.
    #[must_use]
    pub fn hides_at(self) -> Option<DateTime<Utc>> {
        match self {
            Self::Hidden => None,
            Self::Since(at) => at.checked_add_signed(Duration::seconds(LINK_SHOWN_SECS)),
        }
    }
}

/// Password-equivalent guidance below a pairing link needs a warning notice.
pub const PAIRING_WARNING: crate::alert::Variant = crate::alert::Variant::Warning;
