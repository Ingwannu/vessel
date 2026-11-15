//! The source hierarchy extends the published menu, with one outward path.
//! Pointer flyouts and compact pushed pages must never keep separate stacks.
mod navigation;
pub(super) use navigation::Path;
pub use navigation::{PageDepth, PageId};

#[cfg(test)]
mod state_tests;
