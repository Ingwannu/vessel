//! Ephemeral browser updates, deliberately separate from durable session records.
//!
//! These values are staged before stream transport/engine wiring. A PNG payload
//! or descriptor is not runtime/paint qualification. Frames and direct human
//! input must never enter EventRecord, session history, diagnostics or layout.
use super::{BrowserFailure, BrowserId, BrowserRevision, BrowserTab, ScreenshotPng, ViewportSize};
use crate::WorkspaceId;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;

/// Update order is distinct from the document/viewport revision used for input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
pub struct BrowserUpdateSeq(NonZeroU64);
impl BrowserUpdateSeq {
    /// The first update of a newly opened watch; zero is never a sentinel.
    pub const FIRST: Self = Self(NonZeroU64::MIN);
    /// An exhausted watch must end instead of replaying an old sequence.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
    }
    /// Explicit transport ordering/capture edge.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}
/// Effective CSS viewport accompanies the same actual image/document revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserFrame {
    /// Must match the watch's admitted resource; not a CDP target handle.
    pub browser_id: BrowserId,
    /// A frame cannot silently retarget another workspace.
    pub workspace: WorkspaceId,
    /// Context used for display-to-page input, not a frame-rate counter.
    pub revision: BrowserRevision,
    /// Distinct from the PNG's physical raster dimensions and native pane bounds.
    pub viewport: ViewportSize,
    /// Bounded transport/header data; the actual decoder must also validate pixels.
    pub image: ScreenshotPng,
}
/// A watch cannot recover a missed frame by paging the durable agent log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum BrowserWatchEnd {
    /// The resource was closed; its identity is not reused.
    Closed,
    /// Profile/device/workspace/service admission was revoked.
    Revoked,
    /// A slow receiver must open a new watch rather than infer continuity.
    Lagged,
    /// Runtime/transport failure is never a successful last page.
    Failed {
        /// No URL, typed text, source or OS path is reflected in this reason.
        failure: BrowserFailure,
    },
}
/// One ordered content/state/end channel, never a SessionEvent wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserUpdate {
    /// Strict watch order is independent of captured input-context revision.
    pub sequence: BrowserUpdateSeq,
    /// Actual state, actual pixels or an explicit end.
    pub change: BrowserChange,
}
/// Initial descriptor admission and every later change keep the same immutable scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserChange {
    /// State comes from the engine/owner, not a navigation-request echo.
    State {
        /// Replacement document/viewport generation, checked before later input.
        revision: BrowserRevision,
        /// Scope/profile must match the watch's opening descriptor.
        tab: BrowserTab,
    },
    /// An active visible or explicitly requested capture, never a fake no-engine frame.
    Frame {
        /// Complete encoded image and its matching CSS/input context.
        frame: BrowserFrame,
    },
    /// No further change is accepted for this watch id.
    Ended {
        /// There is no fictitious durable resume cursor.
        end: BrowserWatchEnd,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watch_order_and_input_context_are_distinct_and_overflow_cannot_wrap() {
        assert!(serde_json::from_str::<BrowserUpdateSeq>("0").is_err());
        let first = BrowserUpdateSeq::FIRST;
        assert_eq!(first.get(), 1);
        assert_eq!(first.next().unwrap().get(), 2);
        let last: BrowserUpdateSeq = serde_json::from_str(&u64::MAX.to_string()).unwrap();
        assert_eq!(last.next(), None);
        let update = BrowserUpdate {
            sequence: first,
            change: BrowserChange::Ended {
                end: BrowserWatchEnd::Failed {
                    failure: BrowserFailure::NoEngine,
                },
            },
        };
        let encoded = serde_json::to_string(&update).unwrap();
        assert_eq!(
            serde_json::from_str::<BrowserUpdate>(&encoded).unwrap(),
            update
        );
        assert!(!encoded.contains("resume_from"));
        assert!(serde_json::from_str::<BrowserChange>(r#"{"kind":"future_frame"}"#).is_err());
    }
    #[test]
    fn malformed_frame_bytes_never_become_a_watch_image() {
        let frame = r#"{"browser_id":"00000000-0000-4000-8000-000000000001","workspace":"00000000-0000-4000-8000-000000000002","revision":1,"viewport":{"width":390,"height":844},"image":"bm90LXBuZw=="}"#;
        assert!(serde_json::from_str::<BrowserFrame>(frame).is_err());
    }
}
