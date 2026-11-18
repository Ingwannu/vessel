//! Key and outside-press routing for the one menu renderer; no placement or
//! area action policy.
use super::draw::Layer;
use super::{Direction, MenuHandlers, MenuItem, MenuState, RenderLease, keyboard_choice};
use crate::submenu::Submenus;
use gpui::{App, Context, KeyDownEvent, MouseDownEvent, Window};
use std::rc::Rc;

// Input routing stays separate from surface drawing; both menu entry points
// and extended content dispatch through the same dismissal/selection guard.
pub(super) struct KeyMenu<V> {
    pub(super) handlers: MenuHandlers<V>,
    pub(super) items: Rc<[MenuItem]>,
    pub(super) lease: RenderLease,
    /// The menu's submenus, when it has any: Enter on a row that opens a
    /// page opens it.
    pub(super) submenus: Option<Submenus>,
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
        submenus,
    } = input;
    let Some(key) = MenuKey::of(event) else {
        return;
    };
    let menu = (handlers.state)(view);
    if !menu.accepts(lease) {
        // The dispatch path was captured before interceptors. Letting an old
        // Escape bubble could dismiss its parent dialog underneath opening B.
        // Preserve menu-owned key containment without changing B or its focus.
        cx.stop_propagation();
        return;
    }
    match key {
        MenuKey::Escape => menu.close_and_restore(window, cx),
        MenuKey::Step(direction) => menu.step(items, direction),
        MenuKey::First => {
            menu.highlighted = None;
            menu.step(items, Direction::Next);
        }
        MenuKey::Last => {
            menu.highlighted = None;
            menu.step(items, Direction::Previous);
        }
        MenuKey::Choose => {
            let page = super::selected_index(items, menu.highlighted)
                .and_then(|index| items.get(index))
                .and_then(|item| item.opens.as_ref());
            if let (Some(page), Some(submenus)) = (page, submenus) {
                submenus.open(page, 0);
            } else if let Some(index) = keyboard_choice(menu, items) {
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
    }
    cx.stop_propagation();
    cx.notify();
}

// Parse external key strings once; the closed routing set also defines which
// events an obsolete frame must contain instead of forwarding to its parent.
#[derive(Clone, Copy)]
enum MenuKey {
    Escape,
    Step(Direction),
    First,
    Last,
    Choose,
}
impl MenuKey {
    fn of(event: &KeyDownEvent) -> Option<Self> {
        match event.keystroke.key.as_str() {
            "escape" => Some(Self::Escape),
            "down" | "tab" => Some(Self::Step(Direction::Next)),
            "up" => Some(Self::Step(Direction::Previous)),
            "home" => Some(Self::First),
            "end" => Some(Self::Last),
            "enter" | "space" => Some(Self::Choose),
            _other => None,
        }
    }
}

/// A press outside the menu, and outside its open pages (a press there
/// belongs to the menu): close it. A stale frame's press is contained.
pub(super) fn outside<V: 'static>(
    state: &MenuState,
    get: fn(&mut V) -> &mut MenuState,
    layer: Layer,
    pages: Option<Submenus>,
    cx: &mut Context<'_, V>,
) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
    let lease = state.render_lease();
    cx.listener(move |view: &mut V, _event: &MouseDownEvent, window, cx| {
        if pages
            .as_ref()
            .is_some_and(|pages| pages.contains(window.mouse_position()))
        {
            return;
        }
        if !get(view).accepts(&lease) {
            if layer == Layer::AboveDialog {
                // A stale nested menu still contains this press; forwarding
                // along A's path could dismiss the dialog underneath B.
                cx.stop_propagation();
            }
            return;
        }
        get(view).close_and_restore(window, cx);
        if layer == Layer::AboveDialog {
            cx.stop_propagation();
        }
        cx.notify();
    })
}
