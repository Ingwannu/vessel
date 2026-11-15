//! Shared admission limits live below parser and observation consumers so an
//! accepted command list always fits the observation ordinal representation.

/// The most commands in one approved project setup or teardown list.
pub const MAX_PROJECT_COMMANDS: usize = 32;
