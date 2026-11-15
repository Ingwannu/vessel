//! Bounded requests enter the output actor; final snapshots remain readable after exit.
use super::{CaptureError, FinalScreen, render};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch};
use vessel_proto::{
    TerminalSize,
    terminals::{CaptureOptions, TerminalCapture},
};
const RESPONSE_DEADLINE: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub(in crate::terminals) enum CaptureCommand {
    Capture {
        options: CaptureOptions,
        reply: oneshot::Sender<Result<TerminalCapture, CaptureError>>,
    },
    Resize(TerminalSize),
}
#[derive(Debug, Clone)]
pub(in crate::terminals) struct CaptureEndpoint {
    commands: mpsc::Sender<CaptureCommand>,
    final_screen: watch::Receiver<Option<FinalScreen>>,
    validity: watch::Sender<Validity>,
}
impl CaptureEndpoint {
    pub(super) fn new(
        commands: mpsc::Sender<CaptureCommand>,
        final_screen: watch::Receiver<Option<FinalScreen>>,
    ) -> Self {
        let (validity, _receiver) = watch::channel(Validity::Valid);
        Self {
            commands,
            final_screen,
            validity,
        }
    }
    pub(in crate::terminals) async fn capture(
        &self,
        options: CaptureOptions,
    ) -> Result<TerminalCapture, CaptureError> {
        if *self.validity.borrow() == Validity::Invalid {
            return Err(CaptureError::Unavailable);
        }
        if self.final_screen.borrow().is_some() {
            return self.final_capture(options);
        }
        let (reply, response) = oneshot::channel();
        if self
            .commands
            .try_send(CaptureCommand::Capture { options, reply })
            .is_err()
        {
            return self.final_capture(options);
        }
        match tokio::time::timeout(RESPONSE_DEADLINE, response).await {
            Ok(Ok(result)) => {
                if *self.validity.borrow() == Validity::Invalid {
                    Err(CaptureError::Unavailable)
                } else {
                    result
                }
            }
            Ok(Err(_closed)) => self.final_capture(options),
            Err(_elapsed) => Err(CaptureError::Timeout),
        }
    }
    pub(in crate::terminals) fn resize(&self, size: TerminalSize) -> Result<(), CaptureError> {
        self.commands
            .try_send(CaptureCommand::Resize(size))
            .map_err(|_full| CaptureError::Unavailable)
    }
    pub(in crate::terminals) fn invalidate(&self) {
        let _prior = self.validity.send_replace(Validity::Invalid);
    }
    fn final_capture(&self, options: CaptureOptions) -> Result<TerminalCapture, CaptureError> {
        let final_screen = self.final_screen.borrow();
        let snapshot = final_screen.as_ref().ok_or(CaptureError::Unavailable)?;
        match &snapshot.screen {
            Ok(screen) => {
                render::capture(snapshot.terminal, screen.capture(render::range(options)))
            }
            Err(error) => Err(*error),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Validity {
    Valid,
    Invalid,
}
