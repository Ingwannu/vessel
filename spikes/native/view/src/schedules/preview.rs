//! Whether a preview may be asked, and when its cadence may be saved.
//!
//! Instants come only from the node's reply. An older peer builds no
//! request, so the form cannot invent times (ADR 0072).

use chrono::{DateTime, Utc};
use vessel_proto::automation::{Cadence, PreviewCount, PreviewSchedule, SchedulePreview};

use super::{PreviewLane, SchedulePeer};

/// What the form may do with `schedule.preview`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewPlan {
    /// Protocol older than 13, or schedules are not offered. No request.
    OlderPeer,
    /// The cadence is not one this app can ask about.
    Unsupported,
    /// The request. Count is five, the protocol default.
    Request(PreviewSchedule),
}

/// A request only when this peer offers preview and the cadence is `every`
/// or cron. The count is the protocol default of five.
#[must_use]
pub fn plan(peer: &SchedulePeer, cadence: &Cadence, after: Option<DateTime<Utc>>) -> PreviewPlan {
    let SchedulePeer::Ready(ready) = peer else {
        return PreviewPlan::OlderPeer;
    };
    if ready.preview == PreviewLane::Older {
        return PreviewPlan::OlderPeer;
    }
    match cadence {
        Cadence::Every { .. } | Cadence::Cron { .. } => PreviewPlan::Request(PreviewSchedule {
            cadence: cadence.clone(),
            after,
            count: PreviewCount::default(),
        }),
        Cadence::Unknown => PreviewPlan::Unsupported,
    }
}

/// The preview cadence, when it is the form's cadence with the anchor
/// filled in. Any other difference is not reused: `every` is then saved
/// with no anchor and the node fills it.
#[must_use]
pub fn reusable<'a>(form: &Cadence, preview: &'a SchedulePreview) -> Option<&'a Cadence> {
    match (form, &preview.cadence) {
        (
            Cadence::Every {
                seconds: wanted,
                anchor: None,
            },
            Cadence::Every {
                seconds: got,
                anchor: Some(_),
            },
        ) if wanted == got => Some(&preview.cadence),
        (form, got) if form == got => Some(&preview.cadence),
        (
            Cadence::Every { .. } | Cadence::Cron { .. } | Cadence::Unknown,
            Cadence::Every { .. } | Cadence::Cron { .. } | Cadence::Unknown,
        ) => None,
    }
}
