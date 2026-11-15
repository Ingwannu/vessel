//! Workspace keyboard targets from the same post-filter/pin/group projection
//! the sidebar draws. Paseo sidebar-projection.ts + utils/sidebar-shortcuts.ts
//! use full OPEN sections, before each section's ShowMore painting limit.

use super::Step;
/// Source numeric badges apply to the first nine logical workspace targets.
pub const NUMBERED_TARGETS: u8 = 9;
/// A group excluded by collapse, independent of scroll/ShowMore painting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expansion {
    /// All logical placements participate.
    Open,
    /// No workspace in this section is a keyboard target.
    Collapsed,
}
/// One pinned/project/status section, already in the source's display order.
#[derive(Debug, Clone, Copy)]
pub struct Section<'a, T> {
    /// The exact producer-owned workspace identities.
    pub targets: &'a [T],
    /// Collapse, not row-paint truncation.
    pub expansion: Expansion,
}
/// A one-based workspace badge, never zero or above nine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Number(u8);
impl Number {
    /// Admit only source numeric shortcuts 1 through 9 at the key-dispatch edge.
    #[must_use]
    pub const fn new(value: u8) -> Option<Self> {
        if value >= 1 && value <= NUMBERED_TARGETS {
            Some(Self(value))
        } else {
            None
        }
    }
    /// The exact digit to display or dispatch.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}
/// Pure order only. The caller instantiates T with the canonical workspace
/// target type and activates it through the pane owner, never a session id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Targets<T>(Vec<T>);
impl<T: Clone + PartialEq> Targets<T> {
    /// Build once beside row projection; do not reconstruct the order in a
    /// keyboard listener or include managed child-agent session rows.
    #[must_use]
    pub fn from_sections(sections: &[Section<'_, T>]) -> Self {
        Self(
            sections
                .iter()
                .filter(|section| section.expansion == Expansion::Open)
                .flat_map(|section| section.targets.iter().cloned())
                .collect(),
        )
    }
    /// Full open-section order for previous/next navigation.
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.0
    }
    /// The badge for one canonical workspace; only the first nine are numbered.
    #[must_use]
    pub fn number(&self, target: &T) -> Option<Number> {
        let index = self.0.iter().position(|known| known == target)?;
        if index >= usize::from(NUMBERED_TARGETS) {
            return None;
        }
        u8::try_from(index.checked_add(1)?).ok().map(Number)
    }
    /// Select one of the source's one-based numbered targets.
    #[must_use]
    pub fn numbered(&self, number: Number) -> Option<&T> {
        self.0.get(usize::from(number.0).checked_sub(1)?)
    }
    /// Next/previous wraps through the complete logical target list. A missing
    /// selection starts at the first/last target as the reference does.
    #[must_use]
    pub fn relative(&self, current: Option<&T>, step: Step) -> Option<&T> {
        let found = current.and_then(|current| self.0.iter().position(|target| target == current));
        let last = self.0.len().checked_sub(1)?;
        let index = match (found, step) {
            (None, Step::Next) => 0,
            (None, Step::Previous) => last,
            (Some(at), Step::Next) => at.checked_add(1).filter(|next| *next <= last).unwrap_or(0),
            (Some(at), Step::Previous) => at.checked_sub(1).unwrap_or(last),
        };
        self.0.get(index)
    }
}

#[cfg(test)]
mod tests;
