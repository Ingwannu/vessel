//! The original engine survives EOF behind a sealed owner, never a restored decoder.
//!
//! D1 can later add only bounded marker metadata changes to this owner. No feed,
//! resize or scrollback mutation is exposed; final cells and decoder carry stay fixed.
//! A readonly snapshot cannot resolve a marker created after that snapshot was made.
use super::{CaptureError, render};
use std::sync::Mutex;
use vessel_proto::{
    TerminalId,
    terminals::{CaptureOptions, TerminalCapture},
};
use vessel_terminal_screen::Screen;

#[derive(Debug)]
pub(super) struct FinalOwner {
    screen: Mutex<Result<Screen, CaptureError>>,
}
impl FinalOwner {
    pub(super) const fn seal(screen: Result<Screen, CaptureError>) -> Self {
        Self {
            screen: Mutex::new(screen),
        }
    }

    pub(super) fn capture(
        &self,
        terminal: TerminalId,
        options: CaptureOptions,
    ) -> Result<TerminalCapture, CaptureError> {
        let screen = self
            .screen
            .lock()
            .map_err(|_poisoned| CaptureError::Unavailable)?;
        match &*screen {
            Ok(screen) => render::capture(terminal, screen.capture(render::range(options))),
            Err(error) => Err(*error),
        }
    }

    #[cfg(test)]
    pub(super) fn snapshot(&self) -> Result<vessel_terminal_screen::Snapshot, CaptureError> {
        let screen = self
            .screen
            .lock()
            .map_err(|_poisoned| CaptureError::Unavailable)?;
        screen
            .as_ref()
            .map(Screen::snapshot)
            .map_err(|error| *error)
    }
}
