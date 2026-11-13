//! Paseo's complete `SPACING` scale (`styles/theme.ts`), in logical points.
//!
//! [Decision Log]
//! - Purpose: screens name one shared spacing scale instead of copying
//!   otherwise unrelated gap and inset literals into each area.
//! - Constraints: the reference's keys and values must remain directly
//!   comparable; widths and row heights are separate geometry.
//! - Alternatives: only the seven values the first consumer needs, or a
//!   scale private to its area.
//! - Choice: every reference step, with the key in its constant's name
//!   (`S0_5` means `spacing[0.5]`) and the points in its value.
//! - Why: one source can serve independently landing areas without
//!   growing a second interpretation of the same reference scale.
//! - Cost: a reference scale change requires updating the exact-value
//!   check and reviewing the consumers, rather than changing one screen.

/// `spacing[0]`: 0 points.
pub const S0: f32 = 0.0;
/// `spacing[0.5]`: 2 points.
pub const S0_5: f32 = 2.0;
/// `spacing[1]`: 4 points.
pub const S1: f32 = 4.0;
/// `spacing[1.5]`: 6 points.
pub const S1_5: f32 = 6.0;
/// `spacing[2]`: 8 points.
pub const S2: f32 = 8.0;
/// `spacing[3]`: 12 points.
pub const S3: f32 = 12.0;
/// `spacing[3.5]`: 14 points.
pub const S3_5: f32 = 14.0;
/// `spacing[4]`: 16 points.
pub const S4: f32 = 16.0;
/// `spacing[6]`: 24 points.
pub const S6: f32 = 24.0;
/// `spacing[7]`: 28 points.
pub const S7: f32 = 28.0;
/// `spacing[8]`: 32 points.
pub const S8: f32 = 32.0;
/// `spacing[12]`: 48 points.
pub const S12: f32 = 48.0;
/// `spacing[16]`: 64 points.
pub const S16: f32 = 64.0;
/// `spacing[20]`: 80 points.
pub const S20: f32 = 80.0;
/// `spacing[24]`: 96 points.
pub const S24: f32 = 96.0;
/// `spacing[32]`: 128 points.
pub const S32: f32 = 128.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reference_key_has_its_exact_point_value() {
        let steps = [
            ("0", S0),
            ("0.5", S0_5),
            ("1", S1),
            ("1.5", S1_5),
            ("2", S2),
            ("3", S3),
            ("3.5", S3_5),
            ("4", S4),
            ("6", S6),
            ("7", S7),
            ("8", S8),
            ("12", S12),
            ("16", S16),
            ("20", S20),
            ("24", S24),
            ("32", S32),
        ];
        let points = [
            0_u16, 2, 4, 6, 8, 12, 14, 16, 24, 28, 32, 48, 64, 80, 96, 128,
        ];
        for ((key, actual), expected) in steps.into_iter().zip(points) {
            assert_eq!(
                actual.to_bits(),
                f32::from(expected).to_bits(),
                "spacing[{key}]"
            );
        }
    }
}
