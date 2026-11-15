//! One node screen adapter for standalone and session PTYs, independent of their event rings.
//! The shared pure engine is Silent here; this module owns only actor/DTO glue.

mod channel;
mod render;
pub(super) use channel::{CaptureCommand, CaptureEndpoint};
use tokio::sync::{mpsc, watch};
use vessel_proto::{
    TerminalSize,
    terminals::{CaptureOptions, TerminalAddress, TerminalCapture},
};
use vessel_terminal_screen::{Screen, ScreenError, ScrollbackLines, Size, Snapshot};

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
    /// CLI text capture has no session-TUI target or invented standalone id.
    #[error("text capture requires a standalone terminal target")]
    WrongTarget,
}

impl CaptureError {
    /// Keep terminal and session adapters on the same typed wire classification.
    #[must_use]
    pub const fn code(self) -> vessel_proto::ErrorCode {
        match self {
            Self::Geometry | Self::TooLarge | Self::WrongTarget => {
                vessel_proto::ErrorCode::InvalidRequest
            }
            Self::Unavailable | Self::Timeout => vessel_proto::ErrorCode::Unavailable,
        }
    }
}

impl From<ScreenError> for CaptureError {
    fn from(error: ScreenError) -> Self {
        match error {
            ScreenError::Geometry | ScreenError::Scrollback => Self::Geometry,
            ScreenError::CaptureTooLarge => Self::TooLarge,
            ScreenError::Unavailable | ScreenError::ReplyUnavailable => Self::Unavailable,
        }
    }
}
/// The actor's screen/inbox. No other task can mutate this state.
pub(super) struct ScreenAuthority {
    screen: Result<Screen, CaptureError>,
    inbox: mpsc::Receiver<CaptureCommand>,
    final_screen: watch::Sender<Option<FinalScreen>>,
    target: TerminalAddress,
}
#[derive(Debug, Clone)]
struct FinalScreen {
    target: TerminalAddress,
    screen: Result<Snapshot, CaptureError>,
}
impl std::fmt::Debug for ScreenAuthority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenAuthority")
            .field("target", &self.target)
            .finish_non_exhaustive()
    }
}
impl ScreenAuthority {
    pub(super) fn channel(target: TerminalAddress, size: TerminalSize) -> (CaptureEndpoint, Self) {
        let (commands, inbox) = mpsc::channel(CAPTURE_QUEUE);
        let (final_screen, final_receiver) = watch::channel(None);
        (
            CaptureEndpoint::new(target, commands, final_receiver),
            Self {
                screen: geometry(size).and_then(|size| {
                    Screen::new(size, ScrollbackLines::DEFAULT).map_err(Into::into)
                }),
                inbox,
                final_screen,
                target,
            },
        )
    }
    pub(super) fn output(&mut self, bytes: &[u8]) {
        if let Ok(screen) = &mut self.screen
            && let Err(error) = feed(screen, bytes)
        {
            // A consumed prefix is already applied. Fence capture, never replay
            // that prefix or promote this observer to query-response authority.
            self.screen = Err(error);
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
        match (&mut self.screen, geometry) {
            (Ok(screen), Ok(size)) => {
                if let Err(error) = screen.resize(size) {
                    self.screen = Err(error.into());
                }
            }
            (state, Err(error)) => *state = Err(error),
            (Err(_), Ok(_)) => {}
        }
    }
    fn capture(&self, options: CaptureOptions) -> Result<TerminalCapture, CaptureError> {
        let terminal = render::standalone(self.target)?;
        match &self.screen {
            Ok(screen) => render::capture(terminal, screen.capture(render::range(options))),
            Err(error) => Err(*error),
        }
    }
    pub(super) fn finish(&self) {
        let screen = self
            .screen
            .as_ref()
            .map(Screen::snapshot)
            .map_err(|error| *error);
        let _previous = self.final_screen.send_replace(Some(FinalScreen {
            target: self.target,
            screen,
        }));
    }
}
fn geometry(size: TerminalSize) -> Result<Size, CaptureError> {
    Size::new(size.cols, size.rows).map_err(Into::into)
}
fn feed(screen: &mut Screen, mut bytes: &[u8]) -> Result<(), CaptureError> {
    while !bytes.is_empty() {
        let result = screen.feed(bytes);
        let remaining = bytes
            .get(result.consumed.get()..)
            .filter(|remaining| remaining.len() < bytes.len())
            .ok_or(CaptureError::Unavailable)?;
        let replies = result.replies.map_err(CaptureError::from)?;
        if !replies.as_bytes().is_empty() {
            return Err(CaptureError::Unavailable);
        }
        bytes = remaining;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
