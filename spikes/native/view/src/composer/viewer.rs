//! Image fit and pointer transforms, independent of the window toolkit.
//!
//! Paseo's zoomable viewport fits within a presentation cap, but pans
//! against the entire window. Keeping both extents prevents a portrait
//! image from moving off screen just because its fitted box is narrow.

/// Paseo's lightbox presentation limits, independent of the zoom canvas.
pub const MAX_WIDTH: f32 = 960.0;
/// The fitted image's height cap.
pub const MAX_HEIGHT: f32 = 640.0;
/// Room around a fitted image (Paseo spacing[4]).
pub const PADDING: f32 = 16.0;
/// The lightbox never shrinks below fit: Paseo's lightbox passes no
/// `minScale`, so `ZoomableImage`'s default 1 applies (image.tsx:30; 0.25 is
/// only `ZoomableViewport`'s own default, which the image overrides).
pub const MIN_SCALE: f32 = 1.0;
/// Paseo's viewport zoom ceiling.
pub const MAX_SCALE: f32 = 8.0;
/// Toolbar zoom increment, shared by wheel clamping.
pub const ZOOM_IN: f32 = 1.25;
/// Toolbar zoom decrement, shared by wheel clamping.
pub const ZOOM_OUT: f32 = 0.8;
/// Wheel sensitivity in Paseo's web viewport.
pub const WHEEL_RATE: f32 = 0.01;

/// A finite, nonempty image or canvas size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    width: f32,
    height: f32,
}

impl Extent {
    /// Refuse a zero, negative or nonfinite size at the platform edge.
    #[must_use]
    pub fn new(width: f32, height: f32) -> Option<Self> {
        (width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0)
            .then_some(Self { width, height })
    }

    /// Width in logical points (or source pixels before fitting).
    #[must_use]
    pub fn width(self) -> f32 {
        self.width
    }
    /// Height in the same unit as width.
    #[must_use]
    pub fn height(self) -> f32 {
        self.height
    }
}

/// A finite pointer position, relative to the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    x: f32,
    y: f32,
}

impl Position {
    /// Parse platform coordinates before using them in a transform.
    #[must_use]
    pub fn new(x: f32, y: f32) -> Option<Self> {
        (x.is_finite() && y.is_finite()).then_some(Self { x, y })
    }
}

/// Fitted content with its bounded zoom and pan, reset for each image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    content: Extent,
    canvas: Extent,
    fitted: Extent,
    scale: f32,
    x: f32,
    y: f32,
}

#[expect(
    clippy::float_arithmetic,
    reason = "finite image geometry, bounded by validated extents and zoom limits"
)]
impl Viewport {
    /// Start centered at the fitted size.
    #[must_use]
    pub fn new(content: Extent, canvas: Extent) -> Self {
        let available_w = (canvas.width - PADDING * 2.0).clamp(0.0, MAX_WIDTH);
        let available_h = (canvas.height - PADDING * 2.0).clamp(0.0, MAX_HEIGHT);
        // Set the limiting edge exactly rather than multiplying it by a
        // rounded fit ratio, which can overshoot the presentation cap.
        let fitted = if available_w == 0.0 || available_h == 0.0 {
            Extent {
                width: 0.0,
                height: 0.0,
            }
        } else if available_w / content.width <= available_h / content.height {
            Extent {
                width: available_w,
                height: available_w * (content.height / content.width),
            }
        } else {
            Extent {
                width: available_h * (content.width / content.height),
                height: available_h,
            }
        };
        Self {
            content,
            canvas,
            fitted,
            scale: MIN_SCALE,
            x: 0.0,
            y: 0.0,
        }
    }

    /// A resize returns to fit, as Paseo's resize observer does.
    pub fn resize(&mut self, canvas: Extent) {
        if self.canvas != canvas {
            *self = Self::new(self.content, canvas);
        }
    }

    /// Return to the centered fit after zooming or panning.
    pub fn reset(&mut self) {
        self.scale = MIN_SCALE;
        self.x = 0.0;
        self.y = 0.0;
    }

    /// The current image size on the canvas.
    #[must_use]
    pub fn size(self) -> Extent {
        Extent {
            width: self.fitted.width * self.scale,
            height: self.fitted.height * self.scale,
        }
    }

    /// Current zoom relative to fit.
    #[must_use]
    pub fn scale(self) -> f32 {
        self.scale
    }
    /// Left edge of the transformed image.
    #[must_use]
    pub fn left(self) -> f32 {
        (self.canvas.width - self.size().width) / 2.0 + self.x
    }
    /// Top edge of the transformed image.
    #[must_use]
    pub fn top(self) -> f32 {
        (self.canvas.height - self.size().height) / 2.0 + self.y
    }

    /// Zoom around the pointer; invalid wheel input cannot poison geometry.
    pub fn zoom(&mut self, factor: f32, focal: Position) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let next = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        let cx = self.canvas.width / 2.0;
        let cy = self.canvas.height / 2.0;
        self.x = focal.x - cx - ((focal.x - cx - self.x) / self.scale) * next;
        self.y = focal.y - cy - ((focal.y - cy - self.y) / self.scale) * next;
        self.scale = next;
        self.clamp();
    }

    /// Toolbar zoom uses the window center.
    pub fn zoom_center(&mut self, factor: f32) {
        self.zoom(
            factor,
            Position {
                x: self.canvas.width / 2.0,
                y: self.canvas.height / 2.0,
            },
        );
    }

    /// Wheel zoom uses Paseo's bounded exponential step.
    pub fn wheel(&mut self, delta: f32, focal: Position) {
        if delta.is_finite() {
            self.zoom((-delta * WHEEL_RATE).exp().clamp(ZOOM_OUT, ZOOM_IN), focal);
        }
    }

    /// Translate physical scroll direction into the zoom wheel convention.
    pub fn scroll(&mut self, delta: f32, focal: Position) {
        self.wheel(-delta, focal);
    }

    /// A drag moves content only as far as it still covers the canvas.
    pub fn drag(&mut self, from: Position, to: Position) {
        self.x += to.x - from.x;
        self.y += to.y - from.y;
        self.clamp();
    }

    /// A tap outside the actual image dismisses the lightbox.
    #[must_use]
    pub fn contains(self, at: Position) -> bool {
        at.x >= self.left()
            && at.x <= self.left() + self.size().width
            && at.y >= self.top()
            && at.y <= self.top() + self.size().height
    }

    fn clamp(&mut self) {
        let max_x = ((self.size().width - self.canvas.width) / 2.0).max(0.0);
        let max_y = ((self.size().height - self.canvas.height) / 2.0).max(0.0);
        self.x = self.x.clamp(-max_x, max_x);
        self.y = self.y.clamp(-max_y, max_y);
    }
}

#[cfg(test)]
mod tests;
