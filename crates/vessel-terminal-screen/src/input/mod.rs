//! Pure input bytes from explicit modes; no focus, clipboard or delivery authority.
mod encode;
mod key;

use crate::{InputModes, ModeState};
pub use key::Key;

/// A physical modifier, distinct from negotiated terminal switches.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    /// This modifier is not held for this key event.
    #[default]
    Released,
    /// This modifier is held for this key event.
    Pressed,
}
/// The caller resolves platform/IME admission before constructing this snapshot.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
    /// Shift state for this event.
    pub shift: Modifier,
    /// Alt/Option state for this event.
    pub alt: Modifier,
    /// Control state for this event.
    pub control: Modifier,
    /// Platform Meta/Super state for this event.
    pub meta: Modifier,
}
impl Modifiers {
    const fn parameter(self) -> u8 {
        1 + self.shift.bit() + self.alt.bit() * 2 + self.control.bit() * 4 + self.meta.bit() * 8
    }
}
impl Modifier {
    const fn bit(self) -> u8 {
        match self {
            Self::Released => 0,
            Self::Pressed => 1,
        }
    }
}

/// Caller-owned operation admission, independent from any transport's ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputBudget(usize);
impl InputBudget {
    /// Use the caller's remaining byte admission for one atomic input operation.
    #[must_use]
    pub const fn new(bytes: usize) -> Self {
        Self(bytes)
    }
}

/// Fully encoded input; this value makes no claim that a PTY received it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBytes(Vec<u8>);
impl InputBytes {
    /// Borrow bytes for the existing ordered writer; never feed them as output.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    /// Transfer ownership to the caller's ordered write admission.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

/// Refusal preserves the caller's original event/text for an explicit decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InputError {
    /// The external name is not one supported physical key value.
    #[error("terminal key is unsupported; committed text uses the text path")]
    UnsupportedKey,
    /// A whole operation cannot fit its caller-owned byte admission.
    #[error("terminal input exceeds its operation byte budget")]
    Budget,
    /// A bounded encoding allocation could not be admitted.
    #[error("terminal input allocation is unavailable")]
    Unavailable,
}

/// Encode one admitted physical key, without inferring or changing input modes.
pub fn encode_key(
    key: Key,
    modifiers: Modifiers,
    modes: InputModes,
) -> Result<InputBytes, InputError> {
    encode::key(key, modifiers, modes).map(|text| InputBytes(text.into_bytes()))
}

/// Preserve committed text, including composed/supplementary Unicode, byte-for-byte.
pub fn encode_text(text: &str, budget: InputBudget) -> Result<InputBytes, InputError> {
    let mut bytes = allocate(text.len(), budget)?;
    bytes.extend_from_slice(text.as_bytes());
    Ok(InputBytes(bytes))
}

const PASTE_START: &str = "\x1b[200~";
const PASTE_END: &str = "\x1b[201~";
const DEFUSED_END: &str = "[201~";

/// Encode paste under explicit current modes; an embedded end marker cannot escape.
pub fn encode_paste(
    text: &str,
    modes: InputModes,
    budget: InputBudget,
) -> Result<InputBytes, InputError> {
    if modes.bracketed_paste == ModeState::Disabled {
        return encode_text(text, budget);
    }
    let removed = text.matches(PASTE_END).count();
    let length = text
        .len()
        .checked_sub(removed)
        .and_then(|n| n.checked_add(PASTE_START.len() + PASTE_END.len()))
        .ok_or(InputError::Budget)?;
    let mut bytes = allocate(length, budget)?;
    bytes.extend_from_slice(PASTE_START.as_bytes());
    let mut parts = text.split(PASTE_END);
    if let Some(first) = parts.next() {
        bytes.extend_from_slice(first.as_bytes());
    }
    for part in parts {
        bytes.extend_from_slice(DEFUSED_END.as_bytes());
        bytes.extend_from_slice(part.as_bytes());
    }
    bytes.extend_from_slice(PASTE_END.as_bytes());
    Ok(InputBytes(bytes))
}

fn allocate(length: usize, budget: InputBudget) -> Result<Vec<u8>, InputError> {
    if length > budget.0 {
        return Err(InputError::Budget);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| InputError::Unavailable)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
