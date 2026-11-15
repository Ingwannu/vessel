//! Key routing for the one menu renderer; no placement or area action policy.
use super::{Direction, MenuHandlers, MenuItem, RenderLease, keyboard_choice};
use gpui::{Context, KeyDownEvent, Window};
use std::rc::Rc;

// Input routing stays separate from surface drawing; both menu entry points
// and extended content dispatch through the same dismissal/selection guard.
pub(super) struct KeyMenu<V> {
    pub(super) handlers: MenuHandlers<V>,
    pub(super) items: Rc<[MenuItem]>,
    pub(super) lease: RenderLease,
}

pub(super) fn keyboard<V: 'static>(
    view: &mut V,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<'_, V>,
    input: &KeyMenu<V>,
) {
    let KeyMenu {
        handlers,
        items,
        lease,
    } = input;
    let menu = (handlers.state)(view);
    if !menu.accepts(lease) {
        return;
    }
    match event.keystroke.key.as_str() {
        "escape" => menu.close_and_restore(window, cx),
        "down" | "tab" => menu.step(items, Direction::Next),
        "up" => menu.step(items, Direction::Previous),
        "home" => {
            menu.highlighted = None;
            menu.step(items, Direction::Next);
        }
        "end" => {
            menu.highlighted = None;
            menu.step(items, Direction::Previous);
        }
        "enter" | "space" => {
            if let Some(index) = keyboard_choice(menu, items) {
                if !menu.same_cycle(lease) {
                    return;
                }
                menu.restore_after_choice(window, cx);
                if !(handlers.state)(view).same_cycle(lease) {
                    return;
                }
                (handlers.chose)(view, index, window, cx);
            }
        }
        _other => return,
    }
    cx.stop_propagation();
    cx.notify();
}
