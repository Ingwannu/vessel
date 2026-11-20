//! A terminal session's view logic: keys and sizes sent in order, when a
//! view may resize, what the frame says, and the grid those bytes become.

mod duty;
mod frame;
mod grid;
mod keys;
mod paste;
mod sender;
mod types;

#[cfg(test)]
mod tests;

pub use duty::{KeysTyped, OtherAnswerer, QueryDuty, TerminalOrigin, ViewerFocus, query_duty};
pub use frame::{Liveness, ResizeRule, TerminalLog};
pub use grid::{FindBound, FindView, TerminalModel};
pub use keys::{CursorKeys, Key, Mods};
pub use paste::{NodePlatform, quote_path};
pub use sender::{MAX_PENDING, Outgoing, Taken, TerminalSender};
pub use types::{
    Attrs, Cell, CellCount, Colour, Cursor, FEED_BUDGET, Feed, Generation, Glyph, GridSize, Place,
    Polarity, Rule, ScrollbackLines, Selection, Slant, Snapshot, TerminalError, Visibility, Weight,
};
