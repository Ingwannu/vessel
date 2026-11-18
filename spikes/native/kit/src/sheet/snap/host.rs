//! A bottom sheet's drag as its host keeps it: the dialog's sheet (which
//! had it first), the menu's and the list's (TRACKER 799,
//! `docs/plans/wp-sheet-drag.md`). One engine, one pointer adapter and one
//! release rule; a host says only which height the positions are measured
//! against: the window's (the dialog's authored stops) or its own panel's
//! (a content-sized sheet, Paseo's dynamic sizing: one open stop).

use super::listener::{OnRelease, listener};
use super::{Frame, Geometry, Movement, Origin, Release, SnapPoints, State, Stop};
use gpui::{AnyElement, App, Bounds, Context, Pixels, ScrollHandle, Window, px};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};

#[derive(Debug)]
struct Active {
    points: SnapPoints,
    motion: Rc<RefCell<State>>,
}

impl Active {
    /// `slide_in`: present from below (the dialog's); else start at rest at
    /// its open stop (the menu and list sheets appear at once, as before).
    fn new(points: SnapPoints, slide_in: bool) -> Self {
        let mut motion = State::with_points(points);
        if slide_in {
            motion.present(Instant::now());
        } else {
            motion.retain_compact();
        }
        Self {
            points,
            motion: Rc::new(RefCell::new(motion)),
        }
    }
}

/// One sheet's drag, kept by its host beside its open state and cleared
/// when it closes.
#[derive(Debug, Default)]
pub(crate) struct SheetState {
    active: RefCell<Option<Active>>,
    /// The panel as last drawn: a content-sized sheet's own height.
    panel: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// The body's scroll, which a content pan hands off to; the host's
    /// scrolling list tracks this same handle.
    scroll: RefCell<ScrollHandle>,
    /// The last sampled frame, for the parts drawn apart from the panel
    /// (the dimmer behind it).
    last: Cell<Option<Geometry>>,
}

impl SheetState {
    /// A sheet measured against the window at `points` (the dialog's): it
    /// slides in from below.
    pub(crate) fn frame(&self, points: SnapPoints, window: &Window, cx: &App) -> SheetFrame {
        self.sample(points, true, window.viewport_size().height, window, cx)
    }

    /// A content-sized sheet: one open stop at its own height, measured
    /// where [`Self::panel_probe`] was last drawn; it starts at rest and
    /// cannot be dragged before its first frame is measured.
    pub(crate) fn frame_content(&self, window: &Window, cx: &App) -> SheetFrame {
        let height = self.panel.get().map_or(px(0.0), |panel| panel.size.height);
        self.sample(SnapPoints::CONTENT, false, height, window, cx)
    }

    /// Put inside a content-sized sheet's panel to measure it.
    pub(crate) fn panel_probe(&self) -> AnyElement {
        crate::picker::anchor_probe(&self.panel)
    }

    /// The panel where it was last measured (tests find its handle there).
    #[cfg(test)]
    pub(crate) fn panel_bounds(&self) -> Option<Bounds<Pixels>> {
        self.panel.get()
    }

    fn sample(
        &self,
        points: SnapPoints,
        slide_in: bool,
        height: Pixels,
        window: &Window,
        cx: &App,
    ) -> SheetFrame {
        let mut retained = self.active.borrow_mut();
        let active = retained.get_or_insert_with(|| Active::new(points, slide_in));
        if active.points != points {
            *active = Active::new(points, slide_in);
        }
        let sampled = active
            .motion
            .borrow_mut()
            .frame(height, Instant::now(), cx.reduce_motion());
        let motion = Rc::clone(&active.motion);
        drop(retained);
        if matches!(sampled, Frame::PendingLayout) && !slide_in {
            // A content-sized sheet is measured where it is drawn: draw it
            // once more so its drag knows its height.
            window.request_animation_frame();
        }
        let geometry = match sampled {
            Frame::Visible(geometry) => Some(geometry),
            Frame::PendingLayout | Frame::Closed => None,
        };
        if geometry.is_some_and(|geometry| geometry.movement == Movement::Animating)
            && !cx.reduce_motion()
        {
            window.request_animation_frame();
        }
        self.last.set(geometry);
        SheetFrame {
            geometry,
            motion,
            scroll: self.scroll(),
        }
    }

    /// The body's scroll handle (a content-sized list tracks it).
    pub(crate) fn scroll(&self) -> ScrollHandle {
        self.scroll.borrow().clone()
    }

    /// The dimmer's share as last sampled: all of it at rest or unmeasured.
    pub(crate) fn backdrop(&self) -> f32 {
        self.last.get().map_or(1.0, |geometry| geometry.backdrop)
    }

    /// How far the panel is drawn below its open stop, as last sampled.
    pub(crate) fn offset(&self) -> Pixels {
        self.last.get().map_or(px(0.0), |geometry| geometry.top)
    }

    /// Forget the drag (the sheet closed): the next opening starts afresh.
    pub(crate) fn clear(&self) {
        let _old = self.active.borrow_mut().take();
        self.panel.set(None);
        self.last.set(None);
        *self.scroll.borrow_mut() = ScrollHandle::new();
    }
}

/// One frame of a sheet: where to draw it, and its gesture's parts.
pub(crate) struct SheetFrame {
    /// None before its first measured frame (draw it at rest).
    pub(crate) geometry: Option<Geometry>,
    motion: Rc<RefCell<State>>,
    /// The body's scroll, which a content pan hands off to.
    pub(crate) scroll: ScrollHandle,
}

impl SheetFrame {
    /// Close the sheet's owner when a pan ends below its open stop.
    pub(crate) fn on_release<V: 'static>(
        &self,
        close: impl Fn(&mut V, &mut Window, &mut Context<'_, V>) + 'static,
        cx: &Context<'_, V>,
    ) -> OnRelease {
        let motion = Rc::clone(&self.motion);
        Rc::new(
            cx.listener(move |view: &mut V, release: &Release, window, cx| {
                if *release == Release::Pan(Stop::Closed) {
                    // A unit close callback may refuse a pending/stale decision.
                    // Keep this same control visible until its actual-close ACK.
                    motion.borrow_mut().retain_compact();
                    close(view, window, cx);
                }
            }),
        )
    }

    /// The gesture's probe over a handle or a scrollable body.
    pub(crate) fn listener(&self, origin: Origin, on_release: &OnRelease) -> AnyElement {
        listener(&self.motion, origin, &self.scroll, on_release)
    }
}
