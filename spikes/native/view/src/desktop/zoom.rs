//! Per-window zoom follows Electron's half-level steps, with finite endpoints.

const MIN_STEP: i8 = -16;
const MAX_STEP: i8 = 16;
const HALF_LEVEL: f32 = 0.5;
const ZOOM_BASE: f32 = 1.2;

/// Menus and matched shortcuts select the same adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomChange {
    /// Increase by half a level.
    In,
    /// Decrease by half a level.
    Out,
    /// Restore the display's native scale.
    Reset,
}

/// The model stores steps instead of accumulating floating-point drift.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ZoomLevel(i8);

impl ZoomLevel {
    /// Repeated input saturates at the reference's minimum and maximum levels.
    pub fn adjust(&mut self, change: ZoomChange) {
        self.0 = match change {
            ZoomChange::In => self.0.saturating_add(1).min(MAX_STEP),
            ZoomChange::Out => self.0.saturating_sub(1).max(MIN_STEP),
            ZoomChange::Reset => 0,
        };
    }

    /// A bounded relative multiplier, applied to each window's native display scale.
    #[must_use]
    pub fn factor(self) -> f32 {
        ZOOM_BASE.powf(f32::from(self.0).mul_add(HALF_LEVEL, 0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_steps_reverse_reset_and_saturate_without_accumulated_drift() {
        let mut zoom = ZoomLevel::default();
        zoom.adjust(ZoomChange::In);
        assert!((zoom.factor() - ZOOM_BASE.sqrt()).abs() < f32::EPSILON);
        zoom.adjust(ZoomChange::Out);
        assert!((zoom.factor() - 1.0).abs() < f32::EPSILON);
        for change in [ZoomChange::In, ZoomChange::Out] {
            for _ in 0..100 {
                zoom.adjust(change);
            }
            let endpoint = zoom;
            zoom.adjust(change);
            assert_eq!(zoom, endpoint);
            assert!(zoom.factor().is_finite() && zoom.factor() > 0.0);
            zoom.adjust(ZoomChange::Reset);
            assert_eq!(zoom, ZoomLevel::default());
        }
    }
}
