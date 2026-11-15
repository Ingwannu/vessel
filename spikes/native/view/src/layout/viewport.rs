//! Workspace compactness is independent from other shell breakpoints.
use super::LayoutMode;

/// Paseo's `md` starts at 600 points (`constants/layout.ts`).
pub const COMPACT_LAYOUT_WIDTH: f32 = 600.0;

impl LayoutMode {
    /// Resolve the workspace policy once for rendering, placement and commands.
    /// Wide touch platforms still cannot use desktop pane splitting.
    #[must_use]
    pub fn for_width(width: f32, desktop_splits: bool) -> Self {
        if !width.is_finite() || width < COMPACT_LAYOUT_WIDTH {
            Self::Compact
        } else if desktop_splits {
            Self::SplitPanes
        } else {
            Self::SinglePane
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pane_policy_changes_at_source_md_not_the_legacy_shell_phone_width() {
        assert_eq!(LayoutMode::for_width(599.0, true), LayoutMode::Compact);
        for width in [600.0, 650.0, 767.0, 768.0] {
            assert_eq!(LayoutMode::for_width(width, true), LayoutMode::SplitPanes);
            assert_eq!(LayoutMode::for_width(width, false), LayoutMode::SinglePane);
        }
        assert_eq!(LayoutMode::for_width(599.0, false), LayoutMode::Compact);
        for unknown in [f32::NAN, f32::INFINITY] {
            assert_eq!(LayoutMode::for_width(unknown, true), LayoutMode::Compact);
        }
    }
}
