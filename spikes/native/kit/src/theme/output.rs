//! Purpose-specific diff colours beside the existing output wash. Appearance
//! uses its source's full-row tints, not the output panel's semantic washes.
use super::Scheme;
use gpui::{Hsla, rgb};

/// GOAT `output::diff_line`'s existing wash, unchanged by Appearance.
pub(crate) const OUTPUT_WASH: f32 = 0.08;
const PREVIEW_ADDED_ALPHA: f32 = 0.15;
const PREVIEW_REMOVED_ALPHA: f32 = 0.1;

/// Paseo `AppearancePreview`'s exact marker and full-row paint.
#[derive(Debug, Clone, Copy)]
pub struct PreviewDiff {
    /// theme.colors.diffAddition.
    pub addition: Hsla,
    /// theme.colors.diffDeletion.
    pub deletion: Hsla,
    /// Its fixed GitHub-style added row, independent of the app scheme.
    pub added_tint: Hsla,
    /// Its fixed GitHub-style removed row.
    pub removed_tint: Hsla,
}
impl PreviewDiff {
    /// Resolve marker text by scheme; retain the source preview's fixed tints.
    #[must_use]
    pub fn of(scheme: Scheme) -> Self {
        let (addition, deletion) = match scheme {
            Scheme::Light => (rgb(0x0015_803d).into(), rgb(0x00b9_1c1c).into()),
            Scheme::Dark => (rgb(0x004a_de80).into(), rgb(0x00ef_4444).into()),
        };
        Self {
            addition,
            deletion,
            added_tint: Hsla::from(rgb(0x002e_a043)).opacity(PREVIEW_ADDED_ALPHA),
            removed_tint: Hsla::from(rgb(0x00f8_5149)).opacity(PREVIEW_REMOVED_ALPHA),
        }
    }
}
