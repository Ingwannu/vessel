//! Existing field event wiring, shared by one-line and multiline editors.
//!
//! [Decision Log]
//! - Purpose: keep the shared field root within its existing size budget.
//! - Constraint: preserve callback ordering, Enter/IME and focus behavior.
//! - Alternatives: duplicate input wiring for request-scoped editors.
//! - Choice: move the existing Handlers/listen together into this child module;
//!   only parent visibility and imports change, with no behavioral change.
//! - Cost: one source file; every existing constructor and caller stays intact.
use super::{
    SendOn,
    identity::{FieldId, source},
};
use gpui::{Context, Entity, Window};
use gpui_base::input::InputEvent;
use std::{cell::Cell, rc::Rc};

/// Follow a field's events: Enter, edits, and whether it holds the keys.
/// What a field's events call.
pub(super) struct Handlers<V: 'static> {
    pub(super) callbacks: Callbacks<V>,
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
        callbacks,
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
                    callbacks.submit(owner, source(state.entity_id()), text, window, cx);
                }
            }
            InputEvent::Change => {
                if callbacks.changes() {
                    let text = state.read(cx).value().to_string();
                    callbacks.change(owner, source(state.entity_id()), text, cx);
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

// A host may capture one original immutable request; identified callbacks still
// use the canonical engine FieldId and the very same event subscription.
pub(super) type Submit<V> = Box<dyn Fn(&mut V, String, &mut Window, &mut Context<'_, V>)>;

/// Existing callbacks and request-scoped callbacks share the same event path.
pub(super) enum Callbacks<V: 'static> {
    Plain {
        on_submit: Submit<V>,
        on_change: Option<fn(&mut V, String, &mut Context<'_, V>)>,
    },
    Identified {
        on_submit: fn(&mut V, FieldId, String, &mut Window, &mut Context<'_, V>),
        on_change: fn(&mut V, FieldId, String, &mut Context<'_, V>),
    },
}

impl<V: 'static> Callbacks<V> {
    fn changes(&self) -> bool {
        match self {
            Self::Plain { on_change, .. } => on_change.is_some(),
            Self::Identified { .. } => true,
        }
    }

    fn submit(
        &self,
        owner: &mut V,
        id: FieldId,
        text: String,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) {
        match self {
            Self::Plain { on_submit, .. } => on_submit(owner, text, window, cx),
            Self::Identified { on_submit, .. } => on_submit(owner, id, text, window, cx),
        }
    }
    fn change(&self, owner: &mut V, id: FieldId, text: String, cx: &mut Context<'_, V>) {
        match self {
            Self::Plain { on_change, .. } => {
                if let Some(on_change) = on_change {
                    on_change(owner, text, cx);
                }
            }
            Self::Identified { on_change, .. } => on_change(owner, id, text, cx),
        }
    }
}
