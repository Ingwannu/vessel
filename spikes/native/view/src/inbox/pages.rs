//! A cursor window has no arbitrary history-depth ceiling. Refresh reads only
//! the newest page plus the selected older page, never an ever-growing prefix.
use super::Position;
/// At most two pages per refresh, independent of how far back the person goes.
pub const MAX_PAGES: u16 = 2;
/// Current page boundary; the newest page is also read for new notices/counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pages {
    before: Option<Position>,
}
impl Pages {
    /// Named maximum reads needed by this window.
    #[must_use]
    pub const fn get(self) -> u16 {
        if self.before.is_some() { MAX_PAGES } else { 1 }
    }
    /// Node-local exclusive boundary of the selected older page.
    #[must_use]
    pub const fn before(self) -> Option<Position> {
        self.before
    }
    /// Move strictly backward; a faulty continuation cannot loop or grow RAM.
    #[must_use]
    pub fn more(self, next: Position) -> Option<Self> {
        if self.before.is_some_and(|before| next >= before) {
            return None;
        }
        Some(Self { before: Some(next) })
    }
}
