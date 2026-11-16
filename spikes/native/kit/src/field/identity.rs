//! Source identity is checked by a retained form before applying queued events.
//!
//! [Decision Log]
//! - Purpose: an old editor's queued event must not answer its successor.
//! - Constraint: input state and existing callbacks remain unchanged for other
//!   callers; the toolkit already gives every live editor a stable entity id.
//! - Alternatives: capture a question id in dynamically allocated closures or
//!   infer identity from text. The first duplicates callback ownership; the
//!   second confuses equal text in different requests.
//! - Choice: typed `FieldId` callbacks on the existing field, using engine identity.
//! - Cost: one opt-in constructor; plain callers keep their signatures.
use super::{Kind, TextField, events::Callbacks};
use gpui::{Context, EntityId, Window};

/// Stable identity of the real input state; cloned fields keep the same id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldId(EntityId);

impl FieldId {
    /// Namespace retained render state by the real editor, across cards and panes.
    #[must_use]
    pub fn element_id(self, name: &'static str) -> gpui::ElementId {
        (name, self.0).into()
    }
}

impl TextField {
    /// A form field showing `placeholder` while empty. Enter in it calls
    /// `on_submit` on its owner with the text it holds.
    pub fn new<V: 'static>(
        placeholder: &str,
        on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::line(
            placeholder,
            Callbacks::Plain {
                on_submit: Box::new(on_submit),
                on_change: None,
            },
            window,
            cx,
        )
    }
    /// The same, also calling `on_change` with the text after every edit
    /// (to say what is wrong with it beside the field as it is typed).
    pub fn with_change<V: 'static>(
        placeholder: &str,
        on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
        on_change: fn(&mut V, String, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::line(
            placeholder,
            Callbacks::Plain {
                on_submit: Box::new(on_submit),
                on_change: Some(on_change),
            },
            window,
            cx,
        )
    }

    /// A retained form rejects callbacks whose source no longer owns its draft.
    pub fn with_source<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, FieldId, String, &mut Window, &mut Context<'_, V>),
        on_change: fn(&mut V, FieldId, String, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::line(
            placeholder,
            Callbacks::Identified {
                on_submit,
                on_change,
            },
            window,
            cx,
        )
    }

    /// Identity belongs to the engine, never the render position or text.
    #[must_use]
    pub fn id(&self) -> FieldId {
        match &self.kind {
            Kind::Line(state) => FieldId(state.entity_id()),
            Kind::Lines(state, _) => FieldId(state.entity_id()),
        }
    }
}

pub(super) fn source(id: EntityId) -> FieldId {
    FieldId(id)
}

#[cfg(test)]
mod tests;
