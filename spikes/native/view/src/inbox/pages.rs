//! A person expands the loaded prefix explicitly. A named ceiling bounds the
//! amount retained per node while the node's exact counters remain visible.

/// Maximum simultaneously loaded pages per machine; older attention remains
/// on the node and can still be opened through its source conversation.
pub const MAX_PAGES: u16 = 256;

/// Nonzero page budget with a bounded growth path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pages(u16);
impl Default for Pages {
    fn default() -> Self {
        Self(1)
    }
}
impl Pages {
    /// The sequential page reads in one bounded refresh.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
    /// Load one older page without wrapping or growing past the memory budget.
    #[must_use]
    pub fn more(self) -> Option<Self> {
        self.0
            .checked_add(1)
            .filter(|pages| *pages <= MAX_PAGES)
            .map(Self)
    }
}
