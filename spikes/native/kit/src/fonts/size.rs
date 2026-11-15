//! Nonzero physical text units for the generic kit. Preference ranges are
//! owned by view; the kit only requires drawable committed point sizes.
use std::num::NonZeroU8;

/// A nonzero committed text size in logical points, not an arbitrary number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontSize(NonZeroU8);
impl FontSize {
    /// Refuse zero at this generic drawing boundary. A preference adapter
    /// supplies its already range-checked value.
    #[must_use]
    pub const fn new(points: u8) -> Option<Self> {
        match NonZeroU8::new(points) {
            Some(points) => Some(Self(points)),
            None => None,
        }
    }
    /// The unit's validated integer count, for the type-scale atomics.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0.get()
    }
}

/// Independent committed axes. No axis is derived from another live scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSizes {
    /// Navigation, controls and labels.
    pub interface: FontSize,
    /// Chat and Markdown body text, applied absolutely.
    pub content: FontSize,
    /// Code/diff/terminal text, applied absolutely.
    pub code: FontSize,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_cannot_reach_a_drawable_committed_size() {
        assert_eq!(FontSize::new(0), None);
        assert_eq!(FontSize::new(1).unwrap().get(), 1);
    }
}
