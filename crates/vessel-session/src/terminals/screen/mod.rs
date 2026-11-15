//! Persistent screen authority belongs to the output actor, not its bounded event ring.
//! vt100 types stay private here; all callers receive typed Vessel line projections.

mod channel;
mod render;
pub(super) use channel::{CaptureCommand, CaptureEndpoint};
use std::{num::NonZeroU16, sync::Arc};
use tokio::sync::{mpsc, watch};
use vessel_proto::{
    TerminalId, TerminalSize,
    terminals::{CaptureOptions, TerminalCapture},
};

/// Paseo's terminal manager retains 10,000 lines; cell admission can lower this
/// only for unusually wide screens, documented as an explicit resource difference.
const SCROLLBACK_LINES: usize = 10_000;
/// A terminal cannot reserve arbitrarily many screen cells independently of its log.
const MAX_SCREEN_CELLS: usize = 4_000_000;
/// This preserves per-call bounded allocation even for a full retained screen.
const MAX_CAPTURE_BYTES: usize = 1024 * 1024;
const CAPTURE_QUEUE: usize = 8;

/// Screen failures are explicit; missing history never becomes a fabricated screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CaptureError {
    /// The requested geometry exceeds the persistent screen's admission budget.
    #[error("terminal geometry exceeds the screen capture cell budget")]
    Geometry,
    /// A requested full projection exceeds the response allocation budget.
    #[error("terminal capture exceeds its byte budget; request a smaller line range")]
    TooLarge,
    /// Actor shutdown or a saturated request queue has no successful snapshot.
    #[error("terminal screen capture is temporarily unavailable")]
    Unavailable,
    /// A dropped or slow authority cannot hold the CLI indefinitely.
    #[error("terminal screen capture exceeded its response deadline")]
    Timeout,
}

/// The actor's parser/inbox. No other task can mutate this state.
pub(super) struct ScreenAuthority {
    parser: Result<vt100::Parser, CaptureError>,
    inbox: mpsc::Receiver<CaptureCommand>,
    final_screen: watch::Sender<Option<FinalScreen>>,
    terminal: TerminalId,
}
#[derive(Debug, Clone)]
struct FinalScreen {
    terminal: TerminalId,
    screen: Result<Arc<vt100::Screen>, CaptureError>,
}
impl std::fmt::Debug for ScreenAuthority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenAuthority")
            .field("terminal", &self.terminal)
            .finish_non_exhaustive()
    }
}
impl ScreenAuthority {
    pub(super) fn channel(terminal: TerminalId, size: TerminalSize) -> (CaptureEndpoint, Self) {
        let (commands, inbox) = mpsc::channel(CAPTURE_QUEUE);
        let (final_screen, final_receiver) = watch::channel(None);
        (
            CaptureEndpoint::new(commands, final_receiver),
            Self {
                parser: parser(size),
                inbox,
                final_screen,
                terminal,
            },
        )
    }
    pub(super) fn output(&mut self, bytes: &[u8]) {
        if let Ok(parser) = &mut self.parser {
            parser.process(bytes);
        }
    }
    pub(super) async fn next(&mut self) -> Option<CaptureCommand> {
        self.inbox.recv().await
    }
    pub(super) fn command(&mut self, command: CaptureCommand) {
        match command {
            CaptureCommand::Resize(size) => self.resize(size),
            CaptureCommand::Capture { options, reply } => {
                let result = self.capture(options);
                let _delivered = reply.send(result);
            }
        }
    }
    fn resize(&mut self, size: TerminalSize) {
        let geometry = geometry(size);
        match (&mut self.parser, geometry) {
            (Ok(parser), Ok((rows, cols, capacity))) => {
                if parser.screen().scrollback_capacity() > capacity {
                    self.parser = Err(CaptureError::Geometry);
                } else {
                    parser.set_size(rows, cols);
                }
            }
            (state, Err(error)) => *state = Err(error),
            (Err(_), Ok(_)) => {}
        }
    }
    fn capture(&self, options: CaptureOptions) -> Result<TerminalCapture, CaptureError> {
        match &self.parser {
            Ok(parser) => render::capture(self.terminal, parser.screen(), options),
            Err(error) => Err(*error),
        }
    }
    pub(super) fn finish(&self) {
        let screen = self
            .parser
            .as_ref()
            .map(|parser| Arc::new(parser.screen().clone()))
            .map_err(|error| *error);
        let _previous = self.final_screen.send_replace(Some(FinalScreen {
            terminal: self.terminal,
            screen,
        }));
    }
}
fn geometry(size: TerminalSize) -> Result<(NonZeroU16, NonZeroU16, usize), CaptureError> {
    let rows = NonZeroU16::new(size.rows).ok_or(CaptureError::Geometry)?;
    let cols = NonZeroU16::new(size.cols).ok_or(CaptureError::Geometry)?;
    let capacity = MAX_SCREEN_CELLS
        .checked_div(usize::from(cols.get()))
        .and_then(|total| total.checked_sub(usize::from(rows.get())))
        .ok_or(CaptureError::Geometry)?;
    Ok((rows, cols, SCROLLBACK_LINES.min(capacity)))
}
fn parser(size: TerminalSize) -> Result<vt100::Parser, CaptureError> {
    let (rows, cols, capacity) = geometry(size)?;
    Ok(vt100::Parser::new(rows, cols, capacity))
}

#[cfg(test)]
mod tests;
