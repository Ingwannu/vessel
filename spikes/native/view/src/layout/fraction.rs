//! Validated split share prevents NaN in persisted state and keeps equality lawful.
use super::{LayoutError, MIN_SPLIT_SIZE};
use serde::{Deserialize, Serialize};
/// A finite share within the reference split limits, not an unchecked float.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f32", into = "f32")]
pub struct SplitFraction(f32);
// Construction excludes NaN, so exact equality is reflexive for every inhabitant.
impl Eq for SplitFraction {}
impl SplitFraction {
    /// New splits divide the existing area equally.
    pub const HALF: Self = Self(0.5);
    /// Pointer resize clamps finite geometry; malformed values are refused.
    ///
    /// # Errors
    /// Refuses nonfinite shares; finite pointer positions clamp to the split limits.
    pub fn clamped(value: f32) -> Result<Self, LayoutError> {
        if !value.is_finite() {
            return Err(LayoutError::InvalidState);
        }
        Ok(Self(
            value.clamp(MIN_SPLIT_SIZE, MIN_SPLIT_SIZE.mul_add(-1.0, 1.0)),
        ))
    }
    /// Toolkit flex share after the domain has validated it.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}
impl TryFrom<f32> for SplitFraction {
    type Error = LayoutError;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if value.is_finite()
            && (MIN_SPLIT_SIZE..=MIN_SPLIT_SIZE.mul_add(-1.0, 1.0)).contains(&value)
        {
            Ok(Self(value))
        } else {
            Err(LayoutError::InvalidState)
        }
    }
}
impl From<SplitFraction> for f32 {
    fn from(value: SplitFraction) -> Self {
        value.0
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn untrusted_fraction_refuses_nan_infinity_and_out_of_range_instead_of_restoring_them() {
        for value in [f32::NAN, f32::INFINITY, -1.0, 2.0] {
            assert!(SplitFraction::try_from(value).is_err());
        }
        assert_eq!(
            SplitFraction::clamped(-1.0).unwrap(),
            SplitFraction::try_from(MIN_SPLIT_SIZE).unwrap()
        );
        assert_eq!(
            SplitFraction::clamped(2.0).unwrap(),
            SplitFraction::try_from(MIN_SPLIT_SIZE.mul_add(-1.0, 1.0)).unwrap()
        );
    }
}
