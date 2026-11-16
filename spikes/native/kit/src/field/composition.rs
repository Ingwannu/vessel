//! One weak registry for every kit text engine. Shortcut admission must not
//! infer composition from the active route, displayed file or selected session.
//! The registry is shared by fields and code editors; the existing public
//! `TextField` query remains the only shortcut-facing boundary.
use gpui::{App, Entity, EntityInputHandler as _, Focusable as _, WeakEntity, Window};
use gpui_base::input::{EditorState, InputState, TextareaState};

enum Known {
    Line(WeakEntity<InputState>),
    Lines(WeakEntity<TextareaState>),
    Editor(WeakEntity<EditorState>),
}
thread_local! {
    static KNOWN: std::cell::RefCell<Vec<Known>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn remember(field: Known) {
    KNOWN.with(|known| known.borrow_mut().push(field));
}
pub(super) fn line(state: &Entity<InputState>) {
    remember(Known::Line(state.downgrade()));
}
pub(super) fn lines(state: &Entity<TextareaState>) {
    remember(Known::Lines(state.downgrade()));
}
pub(crate) fn editor(state: &Entity<EditorState>) {
    remember(Known::Editor(state.downgrade()));
}
pub(super) fn focused(window: &mut Window, cx: &mut App) -> bool {
    let known = KNOWN.with(|known| std::mem::take(&mut *known.borrow_mut()));
    let mut alive = Vec::with_capacity(known.len());
    let mut composing = false;
    for field in known {
        let marked = match &field {
            Known::Line(weak) => weak.upgrade().map(|state| {
                !composing
                    && state.read(cx).focus_handle(cx).is_focused(window)
                    && state.update(cx, |state, cx| {
                        state.marked_text_range(window, cx).is_some()
                    })
            }),
            Known::Lines(weak) => weak.upgrade().map(|state| {
                !composing
                    && state.read(cx).focus_handle(cx).is_focused(window)
                    && state.update(cx, |state, cx| {
                        state.marked_text_range(window, cx).is_some()
                    })
            }),
            Known::Editor(weak) => weak.upgrade().map(|state| {
                !composing
                    && state.read(cx).focus_handle(cx).is_focused(window)
                    && state.update(cx, |state, cx| {
                        state.marked_text_range(window, cx).is_some()
                    })
            }),
        };
        if let Some(marked) = marked {
            composing |= marked;
            alive.push(field);
        }
    }
    KNOWN.with(|known| {
        // Preserve registrations created by an engine callback during a query.
        let mut known = known.borrow_mut();
        alive.append(&mut known);
        *known = alive;
    });
    composing
}
