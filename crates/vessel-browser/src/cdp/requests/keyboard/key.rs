//! A finite initial physical-key vocabulary. Unknown layout/IME input refuses.
use super::KeyboardError;
use serde::Serialize;
use vessel_proto::browser::RequiredInput;

// F1 has Windows virtual-key code112; subsequent function keys are consecutive.
const FUNCTION_KEY_BASE: u16 = 111;

/// The foreign virtual-key unit is not an input length or Unicode code point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub(super) struct VirtualKey(u16);
/// Only validated finite keys can construct the foreign key/code text.
#[derive(Clone)]
pub(super) struct PhysicalKey {
    pub(super) virtual_key: VirtualKey,
    key: String,
    code: String,
}
impl std::fmt::Debug for PhysicalKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BrowserKey([redacted])")
    }
}
impl PhysicalKey {
    pub(super) fn parse(input: &RequiredInput) -> Result<Self, KeyboardError> {
        let name = input.as_str();
        if name.len() == 1 {
            let value = name
                .as_bytes()
                .first()
                .copied()
                .ok_or(KeyboardError::Unsupported)?;
            if value.is_ascii_alphabetic() {
                let upper = value.to_ascii_uppercase();
                return Ok(Self {
                    virtual_key: VirtualKey(u16::from(upper)),
                    key: name.to_owned(),
                    code: format!("Key{}", char::from(upper)),
                });
            }
            if value.is_ascii_digit() {
                return Ok(Self {
                    virtual_key: VirtualKey(u16::from(value)),
                    key: name.to_owned(),
                    code: format!("Digit{}", char::from(value)),
                });
            }
        }
        if let Some((key, code, value)) = named(name) {
            return Ok(Self {
                virtual_key: VirtualKey(value),
                key: key.to_owned(),
                code: code.to_owned(),
            });
        }
        if let Some(number) = name.strip_prefix('F').and_then(|n| n.parse::<u16>().ok())
            && (1..=12).contains(&number)
            && name == format!("F{number}")
        {
            return Ok(Self {
                virtual_key: VirtualKey(
                    FUNCTION_KEY_BASE
                        .checked_add(number)
                        .ok_or(KeyboardError::Unsupported)?,
                ),
                key: name.to_owned(),
                code: name.to_owned(),
            });
        }
        Err(KeyboardError::Unsupported)
    }
    /// Move already-validated labels into their one outgoing event.
    pub(super) fn into_parts(self) -> (VirtualKey, String, String) {
        (self.virtual_key, self.key, self.code)
    }
}
fn named(name: &str) -> Option<(&'static str, &'static str, u16)> {
    Some(match name {
        "Backspace" => ("Backspace", "Backspace", 8),
        "Tab" => ("Tab", "Tab", 9),
        "Enter" => ("Enter", "Enter", 13),
        "Escape" => ("Escape", "Escape", 27),
        " " | "Space" => (" ", "Space", 32),
        "PageUp" => ("PageUp", "PageUp", 33),
        "PageDown" => ("PageDown", "PageDown", 34),
        "End" => ("End", "End", 35),
        "Home" => ("Home", "Home", 36),
        "ArrowLeft" => ("ArrowLeft", "ArrowLeft", 37),
        "ArrowUp" => ("ArrowUp", "ArrowUp", 38),
        "ArrowRight" => ("ArrowRight", "ArrowRight", 39),
        "ArrowDown" => ("ArrowDown", "ArrowDown", 40),
        "Insert" => ("Insert", "Insert", 45),
        "Delete" => ("Delete", "Delete", 46),
        "Shift" => ("Shift", "ShiftLeft", 16),
        "Control" => ("Control", "ControlLeft", 17),
        "Alt" => ("Alt", "AltLeft", 18),
        "Meta" => ("Meta", "MetaLeft", 91),
        _ => return None,
    })
}
