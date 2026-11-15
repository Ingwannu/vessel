//! Shared key routing. A final choice returns focus only if its callback has
//! closed this list; drilling into another list retains the current focus.
use super::{ComboHandlers, ComboRow, ComboboxState};
use gpui::{App, Context, KeyDownEvent, Window};
use std::rc::Rc;

/// Arrows (wrapping), Enter, Escape, and Backspace in an empty search to
/// go back, taken before the search row sees them.
pub(super) fn key_handler<V: 'static>(
    rows: &[ComboRow],
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    cx: &mut Context<'_, V>,
) -> impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static {
    let get = handlers.state;
    let lease = state.render_lease();
    let key_rows: Rc<[ComboRow]> = rows.into();
    let chose = Rc::clone(&handlers.chose);
    let back = handlers.back.clone();
    cx.listener(move |view: &mut V, event: &KeyDownEvent, window, cx| {
        let list = get(view);
        if !list.accepts(&lease) {
            // The dispatch path may already include an enclosing dialog. A
            // rejected old Escape must not close that dialog under the new list.
            if matches!(
                event.keystroke.key.as_str(),
                "escape" | "down" | "up" | "enter" | "backspace"
            ) {
                cx.stop_propagation();
            }
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => list.close_and_restore(window, cx),
            "down" => list.step(&key_rows, true),
            "up" => list.step(&key_rows, false),
            "enter" => {
                if let Some(index) = list.choice(&key_rows) {
                    chose(view, index, window, cx);
                    get(view).restore_choice_if_current(&lease, window, cx);
                }
            }
            "backspace" if back.is_some() && list.query(cx).is_empty() => {
                if let Some(back) = &back {
                    back(view, window, cx);
                }
            }
            _other => return,
        }
        cx.stop_propagation();
        cx.notify();
    })
}
