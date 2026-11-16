//! A transparent native layout delegate; the child alone paints and handles input.
use super::{Options, compute};
use gpui::{
    AnyElement, App, Bounds, Display, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Position, Style, Window, point, px,
};

/// One child positioned using its real size against a captured whole trigger.
pub struct Measured {
    trigger: Bounds<Pixels>,
    options: Options,
    child: AnyElement,
}
impl std::fmt::Debug for Measured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Measured")
            .field("trigger", &self.trigger)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}
/// Position one child in full-window logical coordinates during native prepaint.
/// No wrapper focus, input listener, animation, lease or surface is installed.
#[must_use]
pub fn measured(trigger: Bounds<Pixels>, options: Options, child: impl IntoElement) -> Measured {
    Measured {
        trigger,
        options,
        child: child.into_any_element(),
    }
}
impl IntoElement for Measured {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}
impl Element for Measured {
    type RequestLayoutState = LayoutId;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
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
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child_layout: &mut LayoutId,
        window: &mut Window,
        cx: &mut App,
    ) {
        let child = window.layout_bounds(*child_layout);
        let viewport = Bounds::new(point(px(0.0), px(0.0)), window.viewport_size());
        let placed = compute(self.trigger, child.size, viewport, self.options);
        // Anchor the wrapper, as GPUI's own point anchor does. Subtracting
        // the child's origin would cancel its caller-owned relative motion.
        let translation = point(
            px(bounds
                .origin
                .x
                .as_f32()
                .mul_add(-1.0, placed.origin.x.as_f32())),
            px(bounds
                .origin
                .y
                .as_f32()
                .mul_add(-1.0, placed.origin.y.as_f32())),
        );
        // AnyElement already records child focus in Window; this transparent
        // geometry delegate must not focus or blur it. Keep only the report.
        let _child_focus =
            window.with_element_offset(translation, |window| self.child.prepaint(window, cx));
    }
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _child_layout: &mut LayoutId,
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
