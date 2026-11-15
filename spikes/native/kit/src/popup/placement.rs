//! Measured whole-trigger placement, following Paseo ui/anchor.ts:35–128.
//! This module owns geometry only. Callers retain focus, leases and actions;
//! legacy GPUI point anchors remain available for callers that need them.
//!
//! One native child is measured and translated during the same prepaint,
//! keeping painted and input bounds together without a second render cycle.
mod element;
pub use element::{Measured, measured};

use gpui::{Bounds, Pixels, Point, Size, point, px};

/// Horizontal alignment against the captured trigger rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// Align the child's left edge with the trigger's left edge.
    Start,
    /// Align the child's right edge with the trigger's right edge.
    End,
}
/// Requested and actual vertical side of the whole trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Above the trigger's top edge.
    Top,
    /// Below the trigger's bottom edge.
    Bottom,
}
/// Caller-owned logical distances; menus pass four offset and eight edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// Horizontal alignment before edge clamping.
    pub alignment: Alignment,
    /// Preferred side; insufficient space may flip it.
    pub side: Side,
    /// Gap from the chosen trigger edge.
    pub offset: Pixels,
    /// Minimum gap from the viewport edge.
    pub edge: Pixels,
}
/// The final origin and selected side after flip and edge clamping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// Top-left position in the same coordinates as the viewport.
    pub origin: Point<Pixels>,
    /// Actual side before clamping; retained for caller motion metadata.
    pub side: Side,
}

/// Compute from finite native layout measurements, without measuring or state.
/// Flip tests deliberately exclude offset/edge, exactly as the reference.
/// Oversized children align to the low edge instead of using an inverted clamp.
#[must_use]
pub fn compute(
    trigger: Bounds<Pixels>,
    content: Size<Pixels>,
    viewport: Bounds<Pixels>,
    options: Options,
) -> Placed {
    let above = viewport
        .top()
        .as_f32()
        .mul_add(-1.0, trigger.top().as_f32());
    let below = trigger
        .bottom()
        .as_f32()
        .mul_add(-1.0, viewport.bottom().as_f32());
    let height = content.height.as_f32();
    let side = match options.side {
        Side::Bottom if below < height && above > below => Side::Top,
        Side::Top if above < height && below > above => Side::Bottom,
        Side::Top | Side::Bottom => options.side,
    };
    let x = match options.alignment {
        Alignment::Start => trigger.left().as_f32(),
        Alignment::End => content
            .width
            .as_f32()
            .mul_add(-1.0, trigger.right().as_f32()),
    };
    let offset = options.offset.as_f32();
    let y = match side {
        Side::Bottom => offset.mul_add(1.0, trigger.bottom().as_f32()),
        Side::Top => offset.mul_add(-1.0, height.mul_add(-1.0, trigger.top().as_f32())),
    };
    let edge = options.edge.as_f32();
    let low_x = edge.mul_add(1.0, viewport.left().as_f32());
    let low_y = edge.mul_add(1.0, viewport.top().as_f32());
    let high_x = edge.mul_add(
        -1.0,
        content
            .width
            .as_f32()
            .mul_add(-1.0, viewport.right().as_f32()),
    );
    let high_y = edge.mul_add(-1.0, height.mul_add(-1.0, viewport.bottom().as_f32()));
    Placed {
        origin: point(px(x.min(high_x).max(low_x)), px(y.min(high_y).max(low_y))),
        side,
    }
}

#[cfg(test)]
mod tests;
