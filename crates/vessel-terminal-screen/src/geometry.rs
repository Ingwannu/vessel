//! Validate the shared memory/count policy before the engine allocates cells.
use crate::ScreenError;
use std::num::NonZeroU16;

/// Combined admission budget for both screens and retained primary history.
pub const MAX_SCREEN_CELLS: usize = 4_000_000;
/// xterm's smallest supported width; wide-cell operations require a complete pair.
pub const MIN_COLUMNS: u16 = 2;

/// Nonempty geometry that can reserve both primary and alternate screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    rows: NonZeroU16,
    cols: NonZeroU16,
}
impl Size {
    /// Validate geometry before allocating a terminal.
    ///
    /// # Errors
    /// Returns [`ScreenError::Geometry`] for zero or oversized dimensions.
    pub fn new(cols: u16, rows: u16) -> Result<Self, ScreenError> {
        if cols < MIN_COLUMNS {
            return Err(ScreenError::Geometry);
        }
        let value = Self {
            rows: NonZeroU16::new(rows).ok_or(ScreenError::Geometry)?,
            cols: NonZeroU16::new(cols).ok_or(ScreenError::Geometry)?,
        };
        let _capacity = value.capacity(ScrollbackLines::DEFAULT)?;
        Ok(value)
    }
    /// Number of visible columns.
    #[must_use]
    pub const fn cols(self) -> u16 {
        self.cols.get()
    }
    /// Number of visible rows.
    #[must_use]
    pub const fn rows(self) -> u16 {
        self.rows.get()
    }
    pub(crate) const fn engine(self) -> (NonZeroU16, NonZeroU16) {
        (self.rows, self.cols)
    }
    pub(crate) fn capacity(self, limit: ScrollbackLines) -> Result<usize, ScreenError> {
        // Reserve both screens before history; subtraction refuses an impossible size.
        MAX_SCREEN_CELLS
            .checked_div(usize::from(self.cols()))
            .and_then(|rows| rows.checked_sub(usize::from(self.rows()).saturating_mul(2)))
            .map(|rows| rows.min(usize::try_from(limit.get()).unwrap_or(usize::MAX)))
            .ok_or(ScreenError::Geometry)
    }
}

/// The one user-facing retained-history bound, shared by node and native consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbackLines(u32);
impl ScrollbackLines {
    /// Paseo's initial setting.
    pub const DEFAULT: Self = Self(10_000);
    /// Paseo's greatest accepted setting; cell admission may lower effective retention.
    pub const MAX: u32 = 1_000_000;
    /// Validate a setting without silently changing its meaning.
    ///
    /// # Errors
    /// Returns [`ScreenError::Scrollback`] above [`Self::MAX`].
    pub const fn new(lines: u32) -> Result<Self, ScreenError> {
        if lines > Self::MAX {
            Err(ScreenError::Scrollback)
        } else {
            Ok(Self(lines))
        }
    }
    /// Requested number of retained history rows.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
impl Default for ScrollbackLines {
    fn default() -> Self {
        Self::DEFAULT
    }
}
