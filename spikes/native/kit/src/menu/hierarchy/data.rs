//! Additive data API; legacy `MenuItem` and flat handlers remain intact.
//! One private row renderer consumes the legacy item plus this source role.
use super::super::{Layer, MenuItem, MenuState, Width};
use super::navigation::PageId;
use gpui::{AnyElement, Context, ElementId, Pixels, SharedString, Window};
use std::rc::Rc;

/// The chosen value is a check, never an extra selected fill.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Check {
    /// No checked-value semantics or reserved check rail.
    #[default]
    None,
    /// Reserve the leading sixteen-point rail even for an unchecked value.
    Leading(bool),
    /// Only a checked value draws the trailing sixteen-point mark.
    Trailing(bool),
}
/// A source submenu is navigation, not a close-on-activate application action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subtrigger {
    /// Definition opened by this navigation row.
    pub page: PageId,
    /// Current decision value, translated by the caller.
    pub value: Option<SharedString>,
    /// Non-default filter marker, independent of the current value caption.
    pub indicator: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// A row acts or navigates; a submenu cannot carry an action check state.
pub enum Role {
    /// Dispatch a caller action, optionally showing the chosen value.
    Action(Check),
    /// Open a page without dispatching or closing the root.
    Submenu(Subtrigger),
}
impl Default for Role {
    fn default() -> Self {
        Self::Action(Check::None)
    }
}
impl Subtrigger {
    /// Create a branch without a value or active-filter indicator.
    #[must_use]
    pub fn new(page: PageId) -> Self {
        Self {
            page,
            value: None,
            indicator: false,
        }
    }
    /// Whole current-value caption; it may shrink before the chevron.
    #[must_use]
    pub fn with_value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Some(value.into());
        self
    }
    /// Show the source six-point active-filter dot.
    #[must_use]
    pub fn with_indicator(mut self, active: bool) -> Self {
        self.indicator = active;
        self
    }
}
/// Caller-prepared shared icon/color content may occupy the same leading rail.
/// Pending/success feedback keeps its existing precedence over that content.
pub struct Entry {
    /// Existing tone, availability, leading glyph, feedback and dismissal.
    pub item: MenuItem,
    /// Action/check versus navigation, interpreted by the shared engine.
    pub role: Role,
    /// Optional shared project/label mark in the same sixteen-point rail.
    pub leading_content: Option<AnyElement>,
}
impl Entry {
    /// Extend a legacy item without changing any of its action defaults.
    #[must_use]
    pub fn new(item: MenuItem) -> Self {
        Self {
            item,
            role: Role::default(),
            leading_content: None,
        }
    }
    /// One selected-value check; selection never adds a second filled claim.
    #[must_use]
    pub fn choice(item: MenuItem, check: Check) -> Self {
        Self {
            item,
            role: Role::Action(check),
            leading_content: None,
        }
    }
    /// A navigation row always remains in the one menu.
    #[must_use]
    pub fn submenu(item: MenuItem, subtrigger: Subtrigger) -> Self {
        Self {
            item,
            role: Role::Submenu(subtrigger),
            leading_content: None,
        }
    }
    /// Rich caller mark without a second row renderer.
    #[must_use]
    pub fn leading_content(mut self, content: AnyElement) -> Self {
        self.leading_content = Some(content);
        self
    }
}
/// A click-only branch opts out of hover without affecting its explicit action.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HoverIntent {
    /// Source default: pointer intent may open and close this page.
    #[default]
    Enabled,
    /// Only explicit activation opens it; while open all hover-close is locked.
    Disabled,
}
/// One caller-defined list page, used by either presentation.
pub struct Page {
    /// Logical UI page identity, shared with its subtrigger.
    pub id: PageId,
    /// The compact Back-header caption.
    pub title: SharedString,
    /// Caller order; action addresses refer to this exact snapshot.
    pub entries: Vec<Entry>,
    /// Pointer intent policy, independent of availability.
    pub hover: HoverIntent,
}
impl Page {
    /// Define a translated page with source-default hover intent.
    #[must_use]
    pub fn new(id: PageId, title: impl Into<SharedString>, entries: Vec<Entry>) -> Self {
        Self {
            id,
            title: title.into(),
            entries,
            hover: HoverIntent::Enabled,
        }
    }
    /// Change only pointer intent, preserving explicit page activation.
    #[must_use]
    pub fn with_hover_intent(mut self, hover: HoverIntent) -> Self {
        self.hover = hover;
        self
    }
}
/// Geometry is explicit so additive source parity cannot silently change the
/// reviewed compact height of existing flat-menu callers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RowProfile {
    /// Existing pointer28/compact44 entry.
    #[default]
    Legacy,
    /// Current source menu-geometry.ts pointer28/compact40.
    Reference,
}
/// Source compact-mode opt-in; wide surfaces always remain popovers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CompactMode {
    /// Legacy/source default: an anchored compact popover.
    #[default]
    Popover,
    /// Content-sized bottom sheet, replacing only its current page.
    Sheet,
}
/// Callback addresses name the page and row; domain targets stay captured by
/// the caller's one action snapshot, never reconstructed from the menu label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowAddress {
    /// None identifies the root; Some identifies the exact submenu definition.
    pub page: Option<PageId>,
    /// Row within the captured page, including disabled positions.
    pub index: usize,
}
/// Additive content entry; published flat `Content` and handlers stay intact.
pub struct Hierarchy {
    /// Unique root surface identity.
    pub id: ElementId,
    /// Caller order; action addresses refer to this exact snapshot.
    pub entries: Vec<Entry>,
    /// Caller definitions; missing pages prune the active outward path.
    pub pages: Vec<Page>,
    /// Legacy content width or explicit reference root width.
    pub width: Width,
    /// Existing screen/dialog ordering and outside-input policy.
    pub layer: Layer,
    /// Compact presentation only; never a global breakpoint change.
    pub compact: CompactMode,
    /// Optional root header; submenu titles live on their definitions.
    pub sheet_title: Option<SharedString>,
}
impl Hierarchy {
    /// Root with source-default compact popover and existing screen layer.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, entries: Vec<Entry>) -> Self {
        Self {
            id: id.into(),
            entries,
            pages: Vec::new(),
            width: Width::Content,
            layer: Layer::Screen,
            compact: CompactMode::Popover,
            sheet_title: None,
        }
    }
}

impl Hierarchy {
    /// Supply all definitions for the one outward path.
    #[must_use]
    pub fn with_pages(mut self, pages: Vec<Page>) -> Self {
        self.pages = pages;
        self
    }
    /// Exact source root width, such as Display232 or workspace260.
    #[must_use]
    pub fn with_width(mut self, width: Pixels) -> Self {
        self.width = Width::Fixed(width);
        self
    }
    /// Reuse the containing screen/dialog boundary.
    #[must_use]
    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }
    /// Explicitly opt into a compact sheet when the source does.
    #[must_use]
    pub fn with_compact_mode(mut self, compact: CompactMode) -> Self {
        self.compact = compact;
        self
    }
    /// Translate the root sheet header at the caller.
    #[must_use]
    pub fn with_sheet_title(mut self, title: impl Into<SharedString>) -> Self {
        self.sheet_title = Some(title.into());
        self
    }
}
/// An addressed action, invoked after the existing dismissal/focus guard.
pub type Chose<V> = Rc<dyn Fn(&mut V, RowAddress, &mut Window, &mut Context<'_, V>)>;

/// Same caller-owned `MenuState`, with an exact page/row callback address.
pub struct Handlers<V> {
    /// Reach the existing owner state; no parallel hierarchy state is supplied.
    pub state: fn(&mut V) -> &mut MenuState,
    /// Dispatch the exact page/row against the caller's retained action snapshot.
    pub chose: Chose<V>,
}
impl<V> std::fmt::Debug for Handlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handlers").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("item", &self.item)
            .field("role", &self.role)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Page")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("entries", &self.entries)
            .field("hover", &self.hover)
            .finish()
    }
}
impl std::fmt::Debug for Hierarchy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hierarchy")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("layer", &self.layer)
            .field("compact", &self.compact)
            .finish_non_exhaustive()
    }
}
