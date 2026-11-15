//! A checked OS PID value, never an ownership or lifetime claim.

use std::fmt;
use std::num::NonZeroU32;

/// One nonzero operating-system PID for observations and process runners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(NonZeroU32);

/// Zero cannot identify an OS process; no syscall should receive it accidentally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("an operating-system process id must be nonzero")]
pub struct InvalidProcessId;

impl ProcessId {
    /// The representation's first valid PID, shared by boundary tests.
    pub const MIN: Self = Self(NonZeroU32::MIN);

    /// Pass the validated value to an OS or legacy numeric-PID API.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for ProcessId {
    type Error = InvalidProcessId;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value).map(Self).ok_or(InvalidProcessId)
    }
}

impl From<NonZeroU32> for ProcessId {
    fn from(value: NonZeroU32) -> Self {
        Self(value)
    }
}

impl From<ProcessId> for u32 {
    fn from(value: ProcessId) -> Self {
        value.get()
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_construction_never_turns_zero_into_a_process() {
        let below = ProcessId::MIN
            .get()
            .checked_sub(ProcessId::MIN.get())
            .unwrap();
        assert_eq!(ProcessId::try_from(below), Err(InvalidProcessId));
        for value in [ProcessId::MIN.get(), 73, NonZeroU32::MAX.get()] {
            let pid = ProcessId::try_from(value).unwrap();
            assert_eq!(pid.get(), value);
            assert_eq!(u32::from(pid), value);
            assert_eq!(pid.to_string(), value.to_string());
        }
    }
}
