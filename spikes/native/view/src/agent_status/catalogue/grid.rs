//! The reference catalogue column rule, independent from any window backend.
const MIN_TILE: f32 = 300.0;
const MAX_COLUMNS: u8 = 4;
/// One finite measured rail, with its spacing token in logical points.
#[derive(Debug, Clone, Copy)]
pub struct Rail {
    width: f32,
    gap: f32,
}
impl Rail {
    /// Parse backend measurement once. Missing/invalid measurements remain
    /// explicitly absent; the first frame then uses one full-width tile.
    #[must_use]
    pub fn measured_points(width: f32, gap: f32) -> Option<Self> {
        (width.is_finite() && gap.is_finite() && width > 0.0 && gap >= 0.0)
            .then_some(Self { width, gap })
    }
}
/// A source-floored tile extent, constructed only by the grid resolver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileWidth(f32);
impl TileWidth {
    /// Logical points at the drawing boundary.
    #[must_use]
    pub const fn points(self) -> f32 {
        self.0
    }
}
/// One column fills the rail; measured multi-column widths are floored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grid {
    /// Includes the unmeasured first frame.
    Single,
    /// Finite measured width, capped at the source's four columns.
    Columns(Columns),
}
/// Only resolve can construct measured column counts and finite widths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Columns {
    count: u8,
    tile_width: f32,
}
impl Columns {
    /// The number of columns that fit, from two to four.
    #[must_use]
    pub const fn count(self) -> u8 {
        self.count
    }
    /// Source-floored finite tile width.
    #[must_use]
    pub const fn tile_width(self) -> TileWidth {
        TileWidth(self.tile_width)
    }
}
impl Grid {
    /// Mirrors `provider-catalog-list.tsx` `columnCountForWidth`, including exact thresholds.
    #[must_use]
    pub fn resolve(measurement: Option<Rail>) -> Self {
        let Some(input) = measurement else {
            return Self::Single;
        };
        for count in (2..=MAX_COLUMNS).rev() {
            let gaps = f32::from(count.saturating_sub(1));
            let threshold = MIN_TILE.mul_add(f32::from(count), input.gap.mul_add(gaps, 0.0));
            if input.width >= threshold {
                let width = input.gap.mul_add(gaps, 0.0).mul_add(-1.0, input.width);
                return Self::Columns(Columns {
                    count,
                    tile_width: width.mul_add(f32::from(count).recip(), 0.0).floor(),
                });
            }
        }
        Self::Single
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_width_thresholds_cap_at_four_and_floor_measured_tiles() {
        for (width, columns, tile) in [
            (299.0, 1, None),
            (611.0, 1, None),
            (612.0, 2, Some(300.0_f32)),
            (923.0, 2, Some(455.0)),
            (924.0, 3, Some(300.0)),
            (1236.0, 4, Some(300.0)),
            (1241.0, 4, Some(301.0)),
            (2000.0, 4, Some(491.0)),
        ] {
            let resolved = Grid::resolve(Rail::measured_points(width, 12.0));
            let (actual, actual_width) = match resolved {
                Grid::Single => (1, None),
                Grid::Columns(columns) => (columns.count, Some(columns.tile_width.to_bits())),
            };
            assert_eq!((actual, actual_width), (columns, tile.map(f32::to_bits)));
        }
    }
    #[test]
    fn unmeasured_invalid_and_narrow_widths_fill_one_row() {
        for width in [0.0, -1.0, f32::NAN, f32::INFINITY, 390.0] {
            assert_eq!(
                Grid::resolve(Rail::measured_points(width, 12.0)),
                Grid::Single
            );
        }
    }
}
