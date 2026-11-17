//! The page's usage choices (ADR 0107): Paseo's usage preferences, shaped
//! exactly like the native app's (`spikes/native/view/src/prefs/usage.rs`):
//! how percentages read, the host the summary and modal follow, and the
//! windows pinned to the sidebar summary. Whether the summary shows is the
//! sidebar's ([`super::SidebarView::usage`]), as Paseo's footer item.
//!
//! [Decision Log]
//! - Purpose: the choices survive `vessel ui` launches (each a new origin)
//!   and mean the same as on the native app.
//! - Choice: one section, one change per field (two tabs keep both edits);
//!   a pin names the node's source and window ids, never a label, so a
//!   renamed account keeps its pins.
//! - Bounds: at most [`MAX_USAGE_PINS`] pins, more than the summary can
//!   show; a longer list does not read and the page gets the default.

use serde::{Deserialize, Serialize};
use vessel_proto::{NodeId, limits::LimitKey, usage_reports::UsageSourceId};

use super::Bounded;

/// The most pinned windows kept.
pub const MAX_USAGE_PINS: usize = 32;

/// How percentages read: used, or remaining.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsageDisplay {
    /// "31% used" (Paseo's default).
    #[default]
    Used,
    /// "69% remaining".
    Remaining,
}

/// One window pinned to the sidebar summary, by the node's ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsagePin {
    /// The report source.
    pub source: UsageSourceId,
    /// The window within it.
    pub window: LimitKey,
}

/// An explicit pin list; an empty one pins nothing.
pub type UsagePins = Bounded<UsagePin, MAX_USAGE_PINS>;

/// The usage section.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsagePreferences {
    /// How percentages read.
    pub display: UsageDisplay,
    /// The host chosen for the summary and modal; none follows the active
    /// machine, then the first one that reports.
    pub host: Option<NodeId>,
    /// None follows the sources' defaults; a list pins exactly those.
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<UsagePin>>"))]
    pub pins: Option<UsagePins>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_section_reads_like_the_native_one() {
        let host = NodeId::random();
        let read: UsagePreferences = serde_json::from_value(serde_json::json!({
            "display": "remaining",
            "host": host,
            "pins": [{ "source": "saved-codex", "window": "general:weekly" }],
        }))
        .unwrap();
        assert_eq!(read.display, UsageDisplay::Remaining);
        assert_eq!(read.host, Some(host));
        assert_eq!(read.pins.as_ref().map(|pins| pins.items().len()), Some(1));
        let empty: UsagePreferences = serde_json::from_value(
            serde_json::json!({ "display": "used", "host": null, "pins": [] }),
        )
        .unwrap();
        assert_eq!(empty.pins.map(|pins| pins.items().len()), Some(0));
        assert_eq!(
            serde_json::to_value(UsagePreferences::default()).unwrap(),
            serde_json::json!({ "display": "used", "host": null, "pins": null })
        );
    }

    #[test]
    fn too_many_pins_do_not_read() {
        let pin = serde_json::json!({ "source": "saved-codex", "window": "weekly" });
        let many = vec![pin; MAX_USAGE_PINS + 1];
        assert!(serde_json::from_value::<UsagePins>(serde_json::json!(many)).is_err());
    }
}
