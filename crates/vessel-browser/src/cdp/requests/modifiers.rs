//! One typed foreign modifier mask shared by pointer and keyboard requests.
use serde::Serialize;
use vessel_proto::browser::{KeyModifiers, ModifierState};

/// CDP masks have separate units even though both occupy one byte.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(transparent)]
pub(super) struct Modifiers(u8);
impl From<KeyModifiers> for Modifiers {
    fn from(value: KeyModifiers) -> Self {
        const ALT: u8 = 1;
        const CONTROL: u8 = 2;
        const META: u8 = 4;
        const SHIFT: u8 = 8;
        let mut bits = 0;
        for (state, bit) in [
            (value.alt, ALT),
            (value.control, CONTROL),
            (value.meta, META),
            (value.shift, SHIFT),
        ] {
            if state == ModifierState::Held {
                bits |= bit;
            }
        }
        Self(bits)
    }
}
