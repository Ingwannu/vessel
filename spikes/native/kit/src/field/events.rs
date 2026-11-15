//! One subscription for legacy callbacks and window-aware form notifications.
//! Existing constructors keep their default ordering; Appearance adds hooks
//! to this same engine stream, never a second editor or subscription.
use super::SendOn;
use gpui::{Context, Entity, Window};
use gpui_base::input::{InputBaseState, InputEvent, InputModeKind};
use std::{cell::Cell, rc::Rc};

// Edit and blur carry the same current-text notification, not two callback
// protocols. Naming it keeps the public hooks readable without an owner bound.
type TextEvent<V> = fn(&mut V, String, &mut Window, &mut Context<'_, V>);

/// Generic notifications from the existing editor. Consumers own validation
/// and decide whether a composing edit should update their local preview.
pub struct FieldEvents<V: 'static> {
    /// A text edit; Window permits an owner to replace filtered text in place.
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

/// Only one change callback shape can be active for an engine.
pub(super) enum Notifications<V: 'static> {
    Legacy(Option<fn(&mut V, String, &mut Context<'_, V>)>),
    Windowed(FieldEvents<V>),
}
impl<V: 'static> Notifications<V> {
    fn edit<M: InputModeKind>(
        &self,
        owner: &mut V,
        state: &Entity<InputBaseState<M>>,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) {
        match self {
            Self::Legacy(Some(callback)) => callback(owner, state.read(cx).value().to_string(), cx),
            Self::Legacy(None) => {}
            Self::Windowed(events) => {
                if let Some(callback) = events.on_edit {
                    callback(owner, state.read(cx).value().to_string(), window, cx);
                }
            }
        }
    }
    fn focus(&self, owner: &mut V, window: &mut Window, cx: &mut Context<'_, V>) {
        if let Self::Windowed(events) = self
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
        if let Self::Windowed(events) = self
            && let Some(callback) = events.on_blur
        {
            callback(owner, state.read(cx).value().to_string(), window, cx);
        }
    }
}

pub(super) struct Handlers<V: 'static> {
    pub(super) on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
    pub(super) notifications: Notifications<V>,
    pub(super) focused: Rc<Cell<bool>>,
    pub(super) send: Option<Rc<Cell<SendOn>>>,
}
impl<V: 'static> Handlers<V> {
    fn handle<M: InputModeKind>(
        &self,
        owner: &mut V,
        state: &Entity<InputBaseState<M>>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) {
        match event {
            InputEvent::PressEnter {
                shift: false,
                secondary,
            } => {
                if self
                    .send
                    .as_ref()
                    .is_some_and(|send| !send.get().sends(*secondary))
                {
                    state.update(cx, |state, cx| state.replace("\n", window, cx));
                } else {
                    (self.on_submit)(owner, state.read(cx).value().to_string(), window, cx);
                }
            }
            InputEvent::Change => self.notifications.edit(owner, state, window, cx),
            InputEvent::Focus => {
                self.focused.set(true);
                cx.notify();
                self.notifications.focus(owner, window, cx);
            }
            InputEvent::Blur => {
                self.focused.set(false);
                cx.notify();
                self.notifications.blur(owner, state, window, cx);
            }
            InputEvent::PressEnter { .. } => {}
        }
    }
}
pub(super) fn listen<V: 'static, M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
    handlers: Handlers<V>,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) {
    cx.subscribe_in(
        state,
        window,
        move |owner, state, event: &InputEvent, window, cx| {
            handlers.handle(owner, state, event, window, cx);
        },
    )
    .detach();
}

#[cfg(test)]
mod tests;
