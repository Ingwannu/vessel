//! Pointer capture shared by snap-sheet handles and scrollable bodies.
//! A press remains an ordinary row click until native drag slop is crossed.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, HitboxBehavior, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ScrollHandle, Window, canvas, point, px,
};

use super::{Origin, Release, State};

/// The host decides what a completed pan-to-close means for its focus/routes.
pub type OnRelease = Rc<dyn Fn(&Release, &mut Window, &mut App)>;

/// Overlay this probe on a handle or scrollable body, never a text field.
/// `scroll` must be the same handle the body's list tracks.
#[must_use]
pub fn listener(
    state: &Rc<RefCell<State>>,
    origin: Origin,
    scroll: &ScrollHandle,
    on_release: &OnRelease,
) -> AnyElement {
    let state = Rc::clone(state);
    let scroll = scroll.clone();
    let on_release = Rc::clone(on_release);
    canvas(
        |bounds, window, _cx| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |bounds, hitbox, window, _cx| {
            let pressing = Rc::clone(&state);
            let press_scroll = scroll.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, _window, _cx| {
                if phase.capture()
                    && event.button == MouseButton::Left
                    && bounds.contains(&event.position)
                {
                    pressing.borrow_mut().press(
                        origin,
                        event.position,
                        px(f32::from(press_scroll.offset().y).mul_add(-1.0, 0.0)),
                        Instant::now(),
                    );
                }
            });
            let moving = Rc::clone(&state);
            let move_scroll = scroll.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if !phase.capture() || event.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                if let Some(offset) = moving.borrow_mut().pan(
                    event.position,
                    move_scroll.max_offset().y,
                    Instant::now(),
                ) {
                    window.capture_pointer(hitbox.id);
                    let old = move_scroll.offset();
                    move_scroll.set_offset(point(old.x, px(f32::from(offset).mul_add(-1.0, 0.0))));
                    cx.stop_propagation();
                    window.refresh();
                }
            });
            let releasing = Rc::clone(&state);
            let released = Rc::clone(&on_release);
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase.capture() && event.button == MouseButton::Left {
                    let release = releasing.borrow_mut().release(Instant::now());
                    if matches!(release, Release::Pan(_)) {
                        window.release_pointer();
                        cx.stop_propagation();
                        released(&release, window, cx);
                        window.refresh();
                    }
                }
            });
        },
    )
    .absolute()
    .inset_0()
    .min_h(px(0.0))
    .into_any_element()
}
