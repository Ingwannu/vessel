//! ADR0102: one always-masked line over the existing engine and subscription.
//! Stored credentials never enter this control. Only explicit replacement input
//! exists in its engine; public diagnostics and events keep that value opaque.
use super::{Callbacks, Editability, FieldId, TextField};
use crate::Tokens;
use gpui::{AnyElement, App, Context, SharedString, Window};
use gpui_base::input::InputAccessibility;

pub(super) enum LinePrivacy {
    Ordinary,
    Secret(SharedString),
}
pub(super) fn accessibility(label: SharedString) -> InputAccessibility {
    InputAccessibility::new(Some(gpui::Role::PasswordInput))
        .with_label(Some(label))
        .with_value_exposed(false)
}
/// Credential presence, never the credential itself or its length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretPresence {
    /// No credential has been stored.
    Absent,
    /// A credential exists elsewhere; no existing value is loaded here.
    Set,
}
/// Already-localized, nonsecret words and the stored-presence fact.
#[derive(Debug, Clone)]
pub struct SecretFieldContent {
    /// Accessible field label; never populate from an entered value.
    pub label: SharedString,
    /// Placeholder while there is no stored credential or replacement.
    pub empty: SharedString,
    /// Placeholder meaning “set”, never a masked copy of the old credential.
    pub set: SharedString,
    /// Caller-owned stored-presence fact.
    pub presence: SecretPresence,
}
/// An explicit new candidate, not a stored credential or a displayable value.
/// No Serialize/Display/Clone implementation is provided by the kit.
pub struct SecretInput(String);
impl std::fmt::Debug for SecretInput {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("SecretInput(<redacted>)")
    }
}
impl SecretInput {
    /// Consume only at the caller's explicit secret-store request boundary.
    /// Never log, format, capture or place the returned value in ordinary state.
    #[must_use]
    pub fn into_value(self) -> String {
        self.0
    }
}
/// Empty input means no replacement; deletion requires a separate owner action.
#[derive(Debug)]
pub enum SecretEdit {
    /// Keep the stored credential unchanged (or keep its absence).
    Unchanged,
    /// Replace it only after explicit, admitted submit by the owning form.
    Replace(SecretInput),
}
impl SecretEdit {
    pub(super) fn from_text(text: String) -> Self {
        if text.is_empty() {
            Self::Unchanged
        } else {
            Self::Replace(SecretInput(text))
        }
    }
}
/// Secure facade; no raw TextField, text getter, stored-value setter or reveal.
#[derive(Clone)]
pub struct SecretField {
    field: TextField,
}
impl std::fmt::Debug for SecretField {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("SecretField(<redacted>)")
    }
}
impl SecretField {
    /// Construct an empty replacement editor on the canonical line factory.
    /// Same FieldId/listen/editability/IME path; no second event subscription.
    #[must_use]
    pub fn new<V: 'static>(
        content: SecretFieldContent,
        on_submit: fn(&mut V, FieldId, SecretEdit, &mut Window, &mut Context<'_, V>),
        on_change: fn(&mut V, FieldId, SecretEdit, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let placeholder = match content.presence {
            SecretPresence::Absent => content.empty,
            SecretPresence::Set => content.set,
        };
        Self {
            field: TextField::line_with_privacy(
                &placeholder,
                Callbacks::Secure {
                    on_submit,
                    on_change,
                },
                LinePrivacy::Secret(content.label),
                window,
                cx,
            ),
        }
    }
    /// Actual editor origin; it is not a form-visit or backend authority token.
    #[must_use]
    pub fn id(&self) -> FieldId {
        self.field.id()
    }
    /// Return to the same engine; no new focus owner is created.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.field.focus(window, cx);
    }
    /// Hold input using the existing engine's read-only admission.
    pub fn set_editability(&self, editability: Editability, cx: &mut App) {
        self.field.set_editability(editability, cx);
    }
    /// Silent navigation reset of the replacement, never a delete instruction.
    pub fn clear(&self, window: &mut Window, cx: &mut App) {
        self.field.clear(window, cx);
    }
    /// Same form chrome and masked engine; copy/cut remain blocked by masking.
    #[must_use]
    pub fn element(&self, tokens: &Tokens, narrow: bool, cx: &mut App) -> AnyElement {
        self.field.element(tokens, narrow, cx)
    }
}
#[cfg(test)]
mod tests;
