//! Existing field event wiring, shared by one-line and multiline editors.
//!
//! [Decision Log]
//! - Purpose: keep the shared field root within its existing size budget.
//! - Constraint: preserve callback ordering, Enter/IME and focus behavior.
//! - Alternatives: duplicate input wiring for request-scoped editors.
//! - Choice: move the existing Handlers/listen together into this child module;
//!   only parent visibility and imports change, with no behavioral change.
//! - Cost: one source file; every existing constructor and caller stays intact.
//!
//! Window-aware Appearance hooks are one additional callback variant here;
//! canonical Plain/Identified callbacks keep their signatures and identity.
//! Focus flags update before notifications, as they do for ordinary fields.
use super::{
    SendOn,
    identity::{FieldId, source},
};
use gpui::{Context, Entity, Window};
use gpui_base::input::{InputBaseState, InputEvent, InputModeKind};
use std::{cell::Cell, rc::Rc};

// Edit and blur carry the same current-text notification, not two callback
// protocols. Naming it keeps the public hooks readable without an owner bound.
type TextEvent<V> = fn(&mut V, String, &mut Window, &mut Context<'_, V>);

/// Generic notifications from the existing editor. Consumers own validation
/// and decide whether a composing edit should update their local preview.
pub struct FieldEvents<V: 'static> {
    /// An engine Change, possibly repeating current text on native commit.
    /// Window permits an owner to replace filtered text in the same engine.
    pub on_edit: Option<TextEvent<V>>,
    /// Focus gained; numeric owners can defer the shared select-all hook.
    pub on_focus: Option<fn(&mut V, &mut Window, &mut Context<'_, V>)>,
    /// Focus lost, with CURRENT text rather than a stale opening snapshot.
    pub on_blur: Option<TextEvent<V>>,
}
// Debug describes installed hooks without requiring the entire view owner
// to implement Debug or exposing function addresses.
impl<V: 'static> std::fmt::Debug for FieldEvents<V> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FieldEvents")
            .field("on_edit", &self.on_edit.is_some())
            .field("on_focus", &self.on_focus.is_some())
            .field("on_blur", &self.on_blur.is_some())
            .finish()
    }
}
impl<V: 'static> Default for FieldEvents<V> {
    fn default() -> Self {
        Self {
            on_edit: None,
            on_focus: None,
            on_blur: None,
        }
    }
}

/// Follow a field's events: Enter, edits, and whether it holds the keys.
/// What a field's events call.
pub(super) struct Handlers<V: 'static> {
    pub(super) callbacks: Callbacks<V>,
    pub(super) focused: Rc<Cell<bool>>,
    /// A composer's send press; `None` for a form field (Enter submits).
    pub(super) send: Option<Rc<Cell<SendOn>>>,
}

pub(super) fn listen<V: 'static, M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
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
                    callbacks.change(owner, source(state.entity_id()), text, window, cx);
                }
            }
            InputEvent::Focus => {
                focused.set(true);
                cx.notify();
                callbacks.focus(owner, window, cx);
            }
            InputEvent::Blur => {
                focused.set(false);
                cx.notify();
                callbacks.blur(owner, state, window, cx);
            }
            InputEvent::PressEnter { .. } => {}
        },
    )
    .detach();
}

/// Existing callbacks and request-scoped callbacks share the same event path.
pub(super) enum Callbacks<V: 'static> {
    Plain {
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        on_change: Option<fn(&mut V, String, &mut Context<'_, V>)>,
    },
    Identified {
        on_submit: fn(&mut V, FieldId, String, &mut Window, &mut Context<'_, V>),
        on_change: fn(&mut V, FieldId, String, &mut Context<'_, V>),
    },
    Windowed {
        on_submit: TextEvent<V>,
        events: FieldEvents<V>,
    },
}

impl<V: 'static> Callbacks<V> {
    fn changes(&self) -> bool {
        match self {
            Self::Plain { on_change, .. } => on_change.is_some(),
            Self::Identified { .. } => true,
            Self::Windowed { events, .. } => events.on_edit.is_some(),
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
            Self::Windowed { on_submit, .. } => on_submit(owner, text, window, cx),
        }
    }
    fn change(
        &self,
        owner: &mut V,
        id: FieldId,
        text: String,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) {
        match self {
            Self::Plain { on_change, .. } => {
                if let Some(on_change) = on_change {
                    on_change(owner, text, cx);
                }
            }
            Self::Identified { on_change, .. } => on_change(owner, id, text, cx),
            Self::Windowed { events, .. } => {
                if let Some(callback) = events.on_edit {
                    callback(owner, text, window, cx);
                }
            }
        }
    }
}

impl<V: 'static> Callbacks<V> {
    fn focus(&self, owner: &mut V, window: &mut Window, cx: &mut Context<'_, V>) {
        if let Self::Windowed { events, .. } = self
            && let Some(callback) = events.on_focus
        {
            callback(owner, window, cx);
        }
    }
    fn blur<M: InputModeKind>(
        &self,
        owner: &mut V,
        state: &Entity<InputBaseState<M>>,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) {
        if let Self::Windowed { events, .. } = self
            && let Some(callback) = events.on_blur
        {
            callback(owner, state.read(cx).value().to_string(), window, cx);
        }
    }
}
// Keep hook construction with its dispatcher, without growing the shared root.
impl super::TextField {
    /// The same single-line engine, with optional edit/focus/blur hooks.
    /// Existing constructors retain their legacy callbacks and defaults.
    pub fn with_events<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        events: FieldEvents<V>,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::line(
            placeholder,
            Callbacks::Windowed { on_submit, events },
            window,
            cx,
        )
    }
}

#[cfg(test)]
mod tests;
