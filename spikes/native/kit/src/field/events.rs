//! Existing field event wiring, shared by one-line and multiline editors.
//!
//! [Decision Log]
//! - Purpose: keep the shared field root within its existing size budget.
//! - Constraint: preserve callback ordering, Enter/IME and focus behavior.
//! - Alternatives: duplicate input wiring for request-scoped editors.
//! - Choice: move the existing Handlers/listen together into this child module;
//!   only parent visibility and imports change, with no behavioral change.
//! - Cost: one source file; every existing constructor and caller stays intact.
use super::SendOn;
use gpui::{Context, Entity, Window};
use gpui_base::input::InputEvent;
use std::{cell::Cell, rc::Rc};

/// Follow a field's events: Enter, edits, and whether it holds the keys.
/// What a field's events call.
pub(super) struct Handlers<V: 'static> {
    pub(super) on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
    pub(super) on_change: Option<fn(&mut V, String, &mut Context<'_, V>)>,
    pub(super) focused: Rc<Cell<bool>>,
    /// A composer's send press; `None` for a form field (Enter submits).
    pub(super) send: Option<Rc<Cell<SendOn>>>,
}

pub(super) fn listen<V: 'static, M: gpui_base::input::InputModeKind>(
    state: &Entity<gpui_base::input::InputBaseState<M>>,
    handlers: Handlers<V>,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) {
    let Handlers {
        on_submit,
        on_change,
        focused,
        send,
    } = handlers;
    cx.subscribe_in(
        state,
        window,
        move |owner, state, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter {
                shift: false,
                secondary,
            } => {
                if send
                    .as_ref()
                    .is_some_and(|send| !send.get().sends(*secondary))
                {
                    // Enter starts a line here (in place of a selection,
                    // as typing does).
                    state.update(cx, |state, cx| state.replace("\n", window, cx));
                } else {
                    let text = state.read(cx).value().to_string();
                    on_submit(owner, text, window, cx);
                }
            }
            InputEvent::Change => {
                if let Some(on_change) = on_change {
                    let text = state.read(cx).value().to_string();
                    on_change(owner, text, cx);
                }
            }
            InputEvent::Focus => {
                focused.set(true);
                cx.notify();
            }
            InputEvent::Blur => {
                focused.set(false);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => {}
        },
    )
    .detach();
}
