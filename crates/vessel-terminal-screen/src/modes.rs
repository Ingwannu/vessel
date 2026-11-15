//! Negotiated input modes come from decoded CSI, never another escape scanner.
use crate::ScreenError;
use vt100::DecodedParameter;

/// Resource ceiling for nested keyboard pushes; overflow refuses without losing saved flags.
pub const MAX_KEYBOARD_STACK: usize = 1024;

/// A negotiated switch, distinct from an unknown or absent frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeState {
    /// The application has not enabled this input protocol.
    #[default]
    Disabled,
    /// The application enabled this input protocol.
    Enabled,
}
/// Raw negotiated flags retain unknown future bits instead of guessing a key encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KittyFlags(u32);
impl From<u32> for KittyFlags {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl KittyFlags {
    /// Validated numeric flags, independent of the vendored decoder's u16 defaults.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
/// The same state accompanies cells and cursor in retained frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputModes {
    /// Current Kitty flags; stack ownership stays with the engine.
    pub kitty: KittyFlags,
    /// ConPTY's optional Win32 input reporting.
    pub win32: ModeState,
    /// DEC application cursor keys, independent from Kitty and Win32.
    pub application_cursor: ModeState,
    /// Bracketed paste admission for the current foreground application.
    pub bracketed_paste: ModeState,
}
impl InputModes {
    /// Match the reference's modified Enter admission without inferring from text.
    #[must_use]
    pub const fn modified_enter(self) -> ModeState {
        if self.kitty.0 > 0 || matches!(self.win32, ModeState::Enabled) {
            ModeState::Enabled
        } else {
            ModeState::Disabled
        }
    }
}
#[derive(Default)]
pub(crate) struct Modes {
    state: InputModes,
    stack: Vec<KittyFlags>,
}
pub(crate) enum ModeAction {
    None,
    Query(KittyFlags),
}
impl Modes {
    pub(crate) const fn state(&self) -> InputModes {
        self.state
    }
    pub(crate) fn csi(
        &mut self,
        params: &vt100::DecodedParams,
        intermediates: &[u8],
        final_byte: char,
    ) -> Result<ModeAction, ScreenError> {
        if params.iter().any(|group| group.len() != 1) {
            return Ok(ModeAction::None);
        }
        match (intermediates, final_byte) {
            ([b'>'], 'u') => {
                let flags = KittyFlags(parameter(params.explicit().next(), 1)?);
                if self.stack.len() >= MAX_KEYBOARD_STACK || self.stack.try_reserve(1).is_err() {
                    return Err(ScreenError::InputModeUnavailable);
                }
                self.stack.push(self.state.kitty);
                self.state.kitty = flags;
            }
            ([b'='], 'u') => {
                let mut values = params.explicit();
                let flags = parameter(values.next(), 0)?;
                let mode = parameter(values.next(), 1)?;
                self.state.kitty = KittyFlags(if mode == 0 { 0 } else { flags });
            }
            ([b'<'], 'u') => self.pop(parameter(params.explicit().next(), 1)?.max(1)),
            ([b'?'], 'u') => return Ok(ModeAction::Query(self.state.kitty)),
            ([b'?'], 'h' | 'l') => self.private(params, final_byte),
            _ => {}
        }
        Ok(ModeAction::None)
    }
    fn pop(&mut self, count: u32) {
        match usize::try_from(count) {
            Ok(count) if count <= self.stack.len() => {
                for _ in 0..count {
                    self.state.kitty = self.stack.pop().unwrap_or_default();
                }
            }
            Ok(_) | Err(_) => {
                self.stack.clear();
                self.state.kitty = KittyFlags::default();
            }
        }
    }

    fn private(&mut self, params: &vt100::DecodedParams, final_byte: char) {
        let mode = if final_byte == 'h' {
            ModeState::Enabled
        } else {
            ModeState::Disabled
        };
        for parameter in params.explicit() {
            match parameter {
                DecodedParameter::Explicit(1) => self.state.application_cursor = mode,
                DecodedParameter::Explicit(2004) => self.state.bracketed_paste = mode,
                DecodedParameter::Explicit(9001) => self.state.win32 = mode,
                DecodedParameter::Omitted
                | DecodedParameter::Explicit(_)
                | DecodedParameter::Overflow => {}
            }
        }
    }
}
fn parameter(value: Option<DecodedParameter>, default: u32) -> Result<u32, ScreenError> {
    match value {
        None | Some(DecodedParameter::Omitted) => Ok(default),
        Some(DecodedParameter::Explicit(value)) => Ok(value),
        Some(DecodedParameter::Overflow) => Err(ScreenError::InputModeUnavailable),
    }
}

#[cfg(test)]
mod tests;
