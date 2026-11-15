//! One native child is measured and translated; it alone paints its surface.
use super::super::snap::{self, Frame, Movement, Percentage, listener::Admission};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Display, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Position, Style, Window, div, point, px,
};
use std::{cell::RefCell, rc::Rc, time::Instant};
/// Transparent measured child; no overlay/focus/input/action authority.
pub struct Fitted {
    child: AnyElement,
    motion: Rc<RefCell<snap::State>>,
    cap: Percentage,
    admit: Admission,
}
impl std::fmt::Debug for Fitted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fitted")
            .field("cap", &self.cap)
            .finish_non_exhaustive()
    }
}
pub(super) fn fitted(
    child: impl IntoElement,
    cap: Percentage,
    window: &Window,
    motion: &Rc<RefCell<snap::State>>,
    admit: &Admission,
) -> Fitted {
    let viewport = window.viewport_size();
    Fitted {
        child: div()
            .w(viewport.width)
            .max_h(px(viewport.height.as_f32().mul_add(cap.fraction(), 0.0)))
            .flex()
            .flex_col()
            .min_h_0()
            .overflow_hidden()
            .child(child)
            .into_any_element(),
        motion: Rc::clone(motion),
        cap,
        admit: Rc::clone(admit),
    }
}
impl IntoElement for Fitted {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}
impl Element for Fitted {
    type RequestLayoutState = LayoutId;
    type PrepaintState = bool;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, LayoutId) {
        let child = self.child.request_layout(window, cx);
        let style = Style {
            position: Position::Absolute,
            display: Display::Flex,
            ..Style::default()
        };
        (window.request_layout(style, [child], cx), child)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child_layout: &mut LayoutId,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        if !(self.admit)(window, cx) {
            return false;
        }
        let child = window.layout_bounds(*child_layout);
        let height = window.viewport_size().height;
        let now = Instant::now();
        let frame = {
            let mut motion = self.motion.borrow_mut();
            if motion.fit_content(child.size.height, height, self.cap, now)
                == snap::Fit::PendingLayout
            {
                return false;
            }
            motion.frame(height, now, cx.reduce_motion())
        };
        let Frame::Visible(geometry) = frame else {
            return false;
        };
        if geometry.movement == Movement::Animating && !cx.reduce_motion() {
            window.request_animation_frame();
        }
        let shift = point(
            px(bounds.origin.x.as_f32().mul_add(-1.0, 0.0)),
            px(bounds
                .origin
                .y
                .as_f32()
                .mul_add(-1.0, geometry.top.as_f32())),
        );
        window.with_element_offset(shift, |window| self.child.prepaint(window, cx));
        true
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut LayoutId,
        visible: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        if *visible && (self.admit)(window, cx) {
            self.child.paint(window, cx);
        }
    }
}
