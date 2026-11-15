//! A single blocking writer serializes debounced saves and the final close snapshot.

use std::sync::mpsc;

use tokio::sync::oneshot;
use vessel_app_view::desktop::window::{SAVE_DELAY, WindowState};

use super::{DesktopError, store::Store};

const QUEUE_CAPACITY: usize = 64;

enum Command {
    Changed(WindowState),
    Final(WindowState, oneshot::Sender<Result<(), DesktopError>>),
}

/// Process-wide handle; only the primary window supplies remembered bounds.
#[derive(Debug)]
pub struct WindowWriter {
    commands: mpsc::SyncSender<Command>,
}

impl WindowWriter {
    /// Start one owner for this file, away from the rendering thread.
    ///
    /// # Errors
    /// Thread startup failures preserve the original OS cause.
    pub fn start(store: Store) -> Result<Self, DesktopError> {
        let (commands, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let _worker = std::thread::Builder::new()
            .name("vessel-window-state".into())
            .spawn(move || run(&store, &receiver))?;
        Ok(Self { commands })
    }

    /// Backpressure is explicit; a full queue never blocks an input handler.
    ///
    /// # Errors
    /// A full or closed worker requires the caller to report failure.
    pub fn changed(&self, state: WindowState) -> Result<(), DesktopError> {
        self.commands
            .try_send(Command::Changed(state))
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => DesktopError::Busy,
                mpsc::TrySendError::Disconnected(_) => DesktopError::Closed,
            })
    }

    /// A final save is queued after all previous writes and acknowledged once done.
    ///
    /// # Errors
    /// Returns file errors or a worker that stopped before acknowledging the save.
    pub fn finalize(
        &self,
        state: WindowState,
    ) -> impl Future<Output = Result<(), DesktopError>> + 'static + use<> {
        let commands = self.commands.clone();
        async move {
            let (done, wait) = oneshot::channel();
            // Sending can wait for capacity, so it runs on a dedicated blocking
            // thread rather than inside GPUI's foreground future.
            let _sender = std::thread::Builder::new()
                .name("vessel-window-final".into())
                .spawn(move || {
                    drop(commands.send(Command::Final(state, done)));
                })?;
            wait.await.map_err(|_closed| DesktopError::Closed)?
        }
    }
}

fn run(store: &Store, receiver: &mpsc::Receiver<Command>) {
    let mut pending = None;
    loop {
        let next = if pending.is_some() {
            receiver.recv_timeout(SAVE_DELAY)
        } else {
            receiver
                .recv()
                .map_err(|_closed| mpsc::RecvTimeoutError::Disconnected)
        };
        match next {
            Ok(Command::Changed(state)) => pending = Some(state),
            Ok(Command::Final(state, done)) => {
                drop(done.send(store.save_window(state)));
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(state) = pending.take()
                    && let Err(error) = store.save_window(state)
                {
                    tracing::warn!(%error,"window state could not be saved");
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(state) = pending
                    && let Err(error) = store.save_window(state)
                {
                    tracing::warn!(%error,"final window state could not be saved");
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn final_snapshot_cannot_be_overwritten_by_earlier_debounced_state() {
        use vessel_app_view::desktop::window::Presentation;
        let root =
            std::env::temp_dir().join(format!("vessel-window-writer-{}", std::process::id()));
        let store = Store::new(root.clone()).unwrap();
        let writer = WindowWriter::start(store.clone()).unwrap();
        writer.changed(WindowState::default()).unwrap();
        let final_state =
            WindowState::new(Some((20.0, 30.0)), 800.0, 600.0, Presentation::Maximized).unwrap();
        writer.finalize(final_state).await.unwrap();
        assert_eq!(store.load_window().unwrap(), Some(final_state));
        assert!(matches!(
            writer.changed(WindowState::default()),
            Err(DesktopError::Closed)
        ));
        std::fs::remove_dir_all(root).unwrap();
    }
}
