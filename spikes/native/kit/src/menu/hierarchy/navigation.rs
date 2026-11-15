//! One path for popover flyout chains and compact pushed pages.
//! Direct port of ui/menu/menu-navigation.ts; callers own page definitions.
use gpui::SharedString;

/// Stable UI page identity, independent of the caller's action/domain target.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageId(SharedString);
impl PageId {
    /// The caller declares the same id on a page and its subtrigger.
    #[must_use]
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self(value.into())
    }
}

/// Root is depth zero; a nested trigger uses its own page's depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageDepth(usize);
impl PageDepth {
    /// Root triggers have no submenu page above them.
    pub const ROOT: Self = Self(0);
    /// A measured page depth from the one outward path.
    #[must_use]
    pub const fn at(index: usize) -> Self {
        Self(index)
    }
}

/// Empty is the root. Both presentations consume this same outward path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Path(Vec<PageId>);
impl Path {
    pub(crate) fn pages(&self) -> &[PageId] {
        &self.0
    }
    pub(crate) fn current(&self) -> Option<&PageId> {
        self.0.last()
    }
    pub(crate) fn contains_at(&self, id: &PageId, depth: PageDepth) -> bool {
        self.0.get(depth.0) == Some(id)
    }
    /// Preserve descendants when the same branch is already open; otherwise
    /// replace this branch and every page it had opened, exactly as the source.
    pub(crate) fn open(&mut self, id: PageId, depth: PageDepth) -> bool {
        if self.contains_at(&id, depth) {
            return false;
        }
        self.0.truncate(depth.0);
        self.0.push(id);
        true
    }
    pub(crate) fn close(&mut self, depth: PageDepth) -> bool {
        if self.0.len() <= depth.0 {
            return false;
        }
        self.0.truncate(depth.0);
        true
    }
    pub(crate) fn back(&mut self) -> bool {
        self.0.pop().is_some()
    }
    /// A removed page invalidates its entire suffix, including a still-known
    /// descendant. No later Back or flyout may address the missing parent.
    pub(crate) fn prune(&mut self, has_page: impl Fn(&PageId) -> bool) -> bool {
        match self.0.iter().position(|id| !has_page(id)) {
            Some(missing) => self.close(PageDepth::at(missing)),
            None => false,
        }
    }
    pub(crate) fn reset(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(name: &'static str) -> PageId {
        PageId::new(name)
    }
    #[test]
    fn shared_menu_path_nests_and_sibling_trigger_replaces_the_whole_branch() {
        let mut path = Path::default();
        assert!(path.current().is_none());
        assert!(path.open(id("show"), PageDepth::ROOT));
        assert!(path.open(id("colors"), PageDepth::at(1)));
        assert_eq!(path.pages(), &[id("show"), id("colors")]);
        let unchanged = path.clone();
        assert!(!path.open(id("show"), PageDepth::ROOT));
        assert_eq!(path, unchanged);
        assert!(path.open(id("grouping"), PageDepth::ROOT));
        assert_eq!(path.pages(), &[id("grouping")]);
        assert!(path.open(id("checks"), PageDepth::at(1)));
        assert!(path.open(id("display"), PageDepth::at(1)));
        assert_eq!(path.pages(), &[id("grouping"), id("display")]);
    }
    #[test]
    fn shared_menu_path_close_back_and_depth_checks_match_the_source() {
        let mut path = Path::default();
        assert!(!path.back());
        assert!(!path.close(PageDepth::ROOT));
        path.open(id("show"), PageDepth::ROOT);
        path.open(id("colors"), PageDepth::at(1));
        assert!(path.contains_at(&id("show"), PageDepth::ROOT));
        assert!(path.contains_at(&id("colors"), PageDepth::at(1)));
        assert!(!path.contains_at(&id("colors"), PageDepth::ROOT));
        assert_eq!(path.current(), Some(&id("colors")));
        assert!(!path.close(PageDepth::at(2)));
        assert!(path.close(PageDepth::at(1)));
        assert_eq!(path.pages(), &[id("show")]);
        assert!(!path.close(PageDepth::at(1)));
        path.open(id("colors"), PageDepth::at(1));
        assert!(path.back());
        assert_eq!(path.current(), Some(&id("show")));
        assert!(path.back());
        assert!(path.current().is_none());
    }
    #[test]
    fn shared_menu_path_missing_definition_prunes_descendants_and_reopens_at_root() {
        let mut path = Path::default();
        path.open(id("list"), PageDepth::ROOT);
        path.open(id("row"), PageDepth::at(1));
        assert!(!path.prune(|_| true));
        assert!(path.prune(|page| page == &id("list")));
        assert_eq!(path.pages(), &[id("list")]);
        path.open(id("row"), PageDepth::at(1));
        assert!(path.prune(|page| page == &id("row")));
        assert!(path.pages().is_empty());
        path.open(id("list"), PageDepth::ROOT);
        path.reset();
        assert!(path.current().is_none());
    }
}
