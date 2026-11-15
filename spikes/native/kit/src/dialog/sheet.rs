//! Dialog's opt-in snap state reuses the existing sheet engine and pointer adapter.

use crate::sheet::snap::listener::{OnRelease, listener};
use crate::sheet::snap::{Frame, Geometry, Movement, Origin, Release, SnapPoints, State, Stop};
use gpui::{AnyElement, App, Context, ScrollHandle, Window};
use std::{cell::RefCell, rc::Rc, time::Instant};

#[derive(Debug)]
struct Active {
    points: SnapPoints,
    motion: Rc<RefCell<State>>,
    scroll: ScrollHandle,
}

impl Active {
    fn new(points: SnapPoints) -> Self {
        let mut motion = State::with_points(points);
        motion.present(Instant::now());
        Self {
            points,
            motion: Rc::new(RefCell::new(motion)),
            scroll: ScrollHandle::new(),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct SheetState(RefCell<Option<Active>>);

impl SheetState {
    pub(super) fn frame(&self, points: SnapPoints, window: &Window, cx: &App) -> SheetFrame {
        let mut retained = self.0.borrow_mut();
        let active = retained.get_or_insert_with(|| Active::new(points));
        if active.points != points {
            *active = Active::new(points);
        }
        let sampled = active.motion.borrow_mut().frame(
            window.viewport_size().height,
            Instant::now(),
            cx.reduce_motion(),
        );
        let geometry = match sampled {
            Frame::Visible(geometry) => Some(geometry),
            Frame::PendingLayout | Frame::Closed => None,
        };
        if geometry.is_some_and(|geometry| geometry.movement == Movement::Animating)
            && !cx.reduce_motion()
        {
            window.request_animation_frame();
        }
        SheetFrame {
            geometry,
            motion: Rc::clone(&active.motion),
            scroll: active.scroll.clone(),
        }
    }

    pub(super) fn clear(&self) {
        let _old = self.0.borrow_mut().take();
    }
}

pub(super) struct SheetFrame {
    pub(super) geometry: Option<Geometry>,
    motion: Rc<RefCell<State>>,
    pub(super) scroll: ScrollHandle,
}

impl SheetFrame {
    pub(super) fn on_release<V: 'static>(
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

    pub(super) fn listener(&self, origin: Origin, on_release: &OnRelease) -> AnyElement {
        listener(&self.motion, origin, &self.scroll, on_release)
    }
}
