//! A bounded viewport for already composed sections, with GPUI-owned
//! wheel handling. Fixed siblings belong outside it, as in Paseo's sidebar.
//!
//! The indexed `VirtualList` remains the choice for virtualized rows. This
//! control deliberately does not flatten another area's renderer into rows,
//! prescribe section heights, or implement scrolling physics.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyElement, Bounds, Div, ElementId, FocusHandle, Pixels, Point, ScrollHandle, Stateful,
    WeakFocusHandle, div, px,
};

/// A section's position in this viewport's current composition. Missing or
/// not-yet-measured positions return no bounds rather than a zero rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionIndex(usize);

impl SectionIndex {
    /// Name a position without imposing an application-specific section set.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// One persistent GPUI scroll handle; clones share the same viewport.
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    handle: ScrollHandle,
    revealed_focus: Rc<RefCell<Option<WeakFocusHandle>>>,
}

impl ScrollState {
    /// Start at the beginning without reading a window or application state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The measured viewport in window coordinates; empty before layout.
    #[must_use]
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.handle.bounds()
    }

    /// GPUI's content offset: vertical values become negative toward the end.
    #[must_use]
    pub fn offset(&self) -> Point<Pixels> {
        self.handle.offset()
    }

    /// A section's natural bounds before this viewport's own scroll offset.
    #[must_use]
    pub fn section_bounds(&self, section: SectionIndex) -> Option<Bounds<Pixels>> {
        self.handle.bounds_for_item(section.0)
    }

    /// A minimal focus/reveal wrapper for an existing target. It supplies no
    /// activation, button semantics or complete row keyboard accessibility.
    /// Revealing is instant; GPUI continues to own wheel motion and clamping.
    #[must_use]
    pub fn focus_target(
        &self,
        id: impl Into<ElementId>,
        focus: &FocusHandle,
        content: impl IntoElement,
    ) -> Stateful<Div> {
        let state = self.clone();
        let target = focus.clone();
        div()
            .child(content)
            .on_children_prepainted(move |children, window, cx| {
                let focused = window.focused(cx).map(|handle| handle.downgrade());
                if *state.revealed_focus.borrow() == focused {
                    return;
                }
                // A retained focus must not pull the wheel back on every frame.
                // Observing focus outside a target permits revealing it again
                // when keyboard navigation subsequently returns to it.
                if target.contains_focused(window, cx) {
                    if let Some(bounds) = children.first().copied() {
                        *state.revealed_focus.borrow_mut() = focused;
                        if state.reveal_bounds(bounds) {
                            window.refresh();
                        }
                    }
                } else {
                    *state.revealed_focus.borrow_mut() = None;
                }
            })
            .id(id)
            .track_focus(focus)
    }

    fn reveal_bounds(&self, target: Bounds<Pixels>) -> bool {
        let offset = self.offset();
        let next = reveal_offset(self.bounds(), target, offset, self.handle.max_offset());
        if next == offset {
            return false;
        }
        self.handle.set_offset(next);
        true
    }
}

/// A single scrolling body of naturally sized sections. Its parent must be
/// bounded; header and footer siblings keep their natural heights outside it.
pub struct ScrollViewport {
    id: ElementId,
    state: ScrollState,
    sections: Vec<AnyElement>,
}

impl std::fmt::Debug for ScrollViewport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScrollViewport")
            .field("id", &self.id)
            .field("sections", &self.sections.len())
            .finish_non_exhaustive()
    }
}

impl ScrollViewport {
    /// Bind the body to state kept by its owner across renders.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, state: &ScrollState) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            sections: Vec::new(),
        }
    }

    /// Preserve an existing section renderer and its natural content height.
    #[must_use]
    pub fn section(mut self, content: impl IntoElement) -> Self {
        self.sections.push(content.into_any_element());
        self
    }

    /// Fill only the available flex space, even when sections exceed it.
    #[must_use]
    pub fn element(self) -> Stateful<Div> {
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.state.handle)
            .children(
                self.sections
                    .into_iter()
                    .map(|section| div().w_full().flex_none().child(section)),
            )
    }
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite on-screen pixel geometry, clamped to GPUI's measured scroll range"
)]
fn reveal_offset(
    viewport: Bounds<Pixels>,
    target: Bounds<Pixels>,
    offset: Point<Pixels>,
    maximum: Point<Pixels>,
) -> Point<Pixels> {
    let y = if target.size.height > viewport.size.height || target.top() < viewport.top() {
        offset.y + viewport.top() - target.top()
    } else if target.bottom() > viewport.bottom() {
        offset.y - (target.bottom() - viewport.bottom())
    } else {
        offset.y
    };
    Point {
        x: offset.x,
        y: y.max(-maximum.y.max(px(0.0))).min(px(0.0)),
    }
}

#[cfg(test)]
mod tests;
