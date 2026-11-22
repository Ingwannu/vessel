//! Lossless conversion of elapsed cadence for display.
use vessel_proto::automation::EverySeconds;

/// Units for an exact, single-unit elapsed cadence label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalUnit {
    /// Exact seconds, for intervals such as ninety seconds.
    Seconds,
    /// Sixty elapsed seconds.
    Minutes,
    /// 3,600 elapsed seconds.
    Hours,
    /// 86,400 elapsed seconds, independent of daylight saving.
    Days,
}

impl IntervalUnit {
    /// Largest to smallest, to choose the first exact unit.
    pub const DESCENDING: [Self; 4] = [Self::Days, Self::Hours, Self::Minutes, Self::Seconds];

    /// Elapsed seconds represented by one unit.
    #[must_use]
    pub const fn seconds(self) -> u32 {
        match self {
            Self::Seconds => 1,
            Self::Minutes => 60,
            Self::Hours => 3_600,
            Self::Days => 86_400,
        }
    }
}

/// A stored interval opened in the largest unit that preserves every second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalParts {
    count: u32,
    unit: IntervalUnit,
}

impl IntervalParts {
    /// Ninety seconds stays ninety seconds; 7,200 seconds opens as two hours.
    #[must_use]
    pub fn of(seconds: EverySeconds) -> Self {
        for unit in IntervalUnit::DESCENDING {
            if seconds.get().checked_rem(unit.seconds()) == Some(0) {
                return Self {
                    count: seconds
                        .get()
                        .checked_div(unit.seconds())
                        .unwrap_or_default(),
                    unit,
                };
            }
        }
        // Seconds divide every representable interval; retaining the original
        // value is the safe total fallback if the unit list ever changes.
        Self {
            count: seconds.get(),
            unit: IntervalUnit::Seconds,
        }
    }

    /// The whole-number input.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.count
    }

    /// The selected unit.
    #[must_use]
    pub const fn unit(self) -> IntervalUnit {
        self.unit
    }
}
