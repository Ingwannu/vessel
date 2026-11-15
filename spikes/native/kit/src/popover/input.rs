//! Fence a retained frame before GPUI or arbitrary child controls handle input.
//!
//! A div's hit-tested capture listener can be occluded by the existing compact
//! dialog scrim. This transparent element delegates every rendering phase and
//! registers frame-scoped capture listeners without another hitbox or surface.
//! It uses the same opening lease as dismissal, not another owner generation.

use super::Lease;
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    KeyDownEvent, KeyEvent, KeyUpEvent, LayoutId, MouseDownEvent, MouseEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, ScrollEvent, Window,
};

pub(super) fn fence(lease: Lease, child: AnyElement) -> AnyElement {
    InputFence { lease, child }.into_any_element()
}

struct InputFence {
    lease: Lease,
    child: AnyElement,
}

impl Element for InputFence {
    type RequestLayoutState = ();
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
        _inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let _focus = self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        mouse::<MouseDownEvent>(&self.lease, window);
        mouse::<MouseUpEvent>(&self.lease, window);
        mouse::<MouseMoveEvent>(&self.lease, window);
        mouse::<ScrollEvent>(&self.lease, window);
        key::<KeyDownEvent>(&self.lease, window);
        key::<KeyUpEvent>(&self.lease, window);
        self.child.paint(window, cx);
    }
}

impl IntoElement for InputFence {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn mouse<E: MouseEvent>(lease: &Lease, window: &mut Window) {
    let lease = lease.clone();
    window.on_mouse_event(move |_: &E, phase, window, cx| {
        if phase.capture() && !lease.0.get() {
            window.prevent_default();
            cx.stop_propagation();
        }
    });
}

fn key<E: KeyEvent>(lease: &Lease, window: &mut Window) {
    let lease = lease.clone();
    window.on_key_event(move |_: &E, phase, window, cx| {
        if phase.capture() && !lease.0.get() {
            window.prevent_default();
            cx.stop_propagation();
        }
    });
}
