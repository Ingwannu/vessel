//! Raw page keys are separate from committed text and never reach the OS keyboard.
mod key;
#[cfg(test)]
mod tests;

use super::modifiers::Modifiers;
use super::{Acknowledged, Method, Request, sealed};
use key::{PhysicalKey, VirtualKey};
use serde::Serialize;
use std::collections::BTreeMap;
use vessel_proto::browser::{KeyAction, KeyModifiers, RequiredInput};

#[derive(Debug, thiserror::Error)]
pub(crate) enum KeyboardError {
    #[error("browser key is not supported by this keyboard adapter")]
    Unsupported,
    #[error("browser key is not pressed in this target")]
    NotPressed,
}
/// The grammar is finite. An unbounded caller string never becomes a retained key.
#[derive(Default)]
pub(crate) struct KeyboardState {
    held: BTreeMap<VirtualKey, PhysicalKey>,
}
impl std::fmt::Debug for KeyboardState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserKeyboard")
            .field("held_count", &self.held.len())
            .finish()
    }
}
impl KeyboardState {
    /// Failed delivery after state changes quarantines the owning target.
    pub(crate) fn event(
        &mut self,
        input: &RequiredInput,
        action: KeyAction,
        modifiers: KeyModifiers,
    ) -> Result<KeyEvent, KeyboardError> {
        let key = PhysicalKey::parse(input)?;
        let repeat = match action {
            KeyAction::Down => self.held.insert(key.virtual_key, key.clone()).is_some(),
            KeyAction::Up => {
                let _previous = self
                    .held
                    .remove(&key.virtual_key)
                    .ok_or(KeyboardError::NotPressed)?;
                false
            }
        };
        Ok(KeyEvent::new(key, action, modifiers, repeat))
    }
    /// Only this target's previously accepted keys produce a release.
    pub(crate) fn release_all(&mut self) -> Vec<KeyEvent> {
        std::mem::take(&mut self.held)
            .into_values()
            .map(|key| KeyEvent::new(key, KeyAction::Up, KeyModifiers::default(), false))
            .collect()
    }
}
#[derive(Debug, Serialize)]
enum Kind {
    #[serde(rename = "rawKeyDown")]
    Down,
    #[serde(rename = "keyUp")]
    Up,
}
/// Only this sealed event can invoke CDP trusted key dispatch. No arbitrary commands field.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyEvent {
    #[serde(rename = "type")]
    kind: Kind,
    key: String,
    code: String,
    windows_virtual_key_code: VirtualKey,
    modifiers: Modifiers,
    auto_repeat: bool,
}
impl std::fmt::Debug for KeyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BrowserKeyEvent([redacted])")
    }
}
impl KeyEvent {
    fn new(
        key: PhysicalKey,
        action: KeyAction,
        modifiers: KeyModifiers,
        auto_repeat: bool,
    ) -> Self {
        let (virtual_key, key, code) = key.into_parts();
        Self {
            kind: match action {
                KeyAction::Down => Kind::Down,
                KeyAction::Up => Kind::Up,
            },
            key,
            code,
            windows_virtual_key_code: virtual_key,
            modifiers: Modifiers::from(modifiers),
            auto_repeat,
        }
    }
}
request!(KeyEvent, Acknowledged, KeyEvent);
