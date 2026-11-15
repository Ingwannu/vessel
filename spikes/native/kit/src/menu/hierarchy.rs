//! The source hierarchy extends the published menu, with one outward path.
//! Pointer flyouts and compact pushed pages must never keep separate stacks.
mod data;
mod navigation;
pub use data::{
    Check, Chose, CompactMode, Entry, Handlers, Hierarchy, HoverIntent, Page, Role, RowAddress,
    RowProfile, Subtrigger,
};
pub(super) use navigation::Path;
pub use navigation::{PageDepth, PageId};

#[cfg(test)]
mod state_tests;

#[cfg(test)]
mod row_tests;
