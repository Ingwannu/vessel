//! Decode bounded snapshots without trusting a peer's public vector fields.
use super::{SetupCommandProgress, SetupProgress};
use crate::workspace::MAX_PROJECT_COMMANDS;
use serde::de::{Error as _, SeqAccess, Visitor};
use std::fmt;

/// Per-command raw output budget, matching Paseo's middle truncation accumulator.
pub const MAX_SETUP_COMMAND_OUTPUT_BYTES: usize = 64 * 1024;
/// Aggregate raw output bound, derived from the single command-list owner.
pub const MAX_SETUP_OUTPUT_BYTES: usize = MAX_PROJECT_COMMANDS * MAX_SETUP_COMMAND_OUTPUT_BYTES;

/// A snapshot cannot be admitted as bounded structured observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidSetupProgress {
    /// More rows than the project parser can approve.
    #[error("setup observation has too many commands")]
    Commands,
    /// Ordinals are not contiguous and in approved order.
    #[error("setup observation command order is invalid")]
    Order,
    /// A command exceeds the shared output budget.
    #[error("setup observation command output exceeds its budget")]
    Output,
    /// Total output exceeds its derived budget.
    #[error("setup observation aggregate output exceeds its budget")]
    Aggregate,
}
fn row(
    row: &SetupCommandProgress,
    position: usize,
    total: usize,
) -> Result<usize, InvalidSetupProgress> {
    let ordinal = position.checked_add(1).ok_or(InvalidSetupProgress::Order)?;
    if ordinal > MAX_PROJECT_COMMANDS {
        return Err(InvalidSetupProgress::Commands);
    }
    if usize::try_from(row.index.get()).ok() != Some(ordinal) {
        return Err(InvalidSetupProgress::Order);
    }
    let bytes = row.output.bytes().len();
    if bytes > MAX_SETUP_COMMAND_OUTPUT_BYTES {
        return Err(InvalidSetupProgress::Output);
    }
    total
        .checked_add(bytes)
        .filter(|total| *total <= MAX_SETUP_OUTPUT_BYTES)
        .ok_or(InvalidSetupProgress::Aggregate)
}
impl SetupProgress {
    /// Validate locally constructed DTOs before admitting them to a renderer.
    /// Wire deserialization applies the same checks while collecting the rows.
    /// # Errors
    /// Rejects count, ordinal/order or output-budget violations.
    pub fn validate(&self) -> Result<(), InvalidSetupProgress> {
        self.commands
            .iter()
            .enumerate()
            .try_fold(0, |total, (position, command)| {
                row(command, position, total)
            })
            .map(|_total| ())
    }
}
pub(super) fn decode_commands<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<SetupCommandProgress>, D::Error> {
    struct Rows;
    impl<'de> Visitor<'de> for Rows {
        type Value = Vec<SetupCommandProgress>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("bounded setup commands in approved order")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            if sequence
                .size_hint()
                .is_some_and(|count| count > MAX_PROJECT_COMMANDS)
            {
                return Err(A::Error::custom(InvalidSetupProgress::Commands));
            }
            let mut commands = Vec::new();
            let mut total = 0;
            while let Some(command) = sequence.next_element::<SetupCommandProgress>()? {
                total = row(&command, commands.len(), total).map_err(A::Error::custom)?;
                commands.push(command);
            }
            Ok(commands)
        }
    }
    decoder.deserialize_seq(Rows)
}

#[cfg(test)]
mod tests;
