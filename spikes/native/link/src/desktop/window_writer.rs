//! A single blocking writer serializes debounced saves and the final close snapshot.
//! Checkpoints preserve later accepted geometry even if their sender runs late.

use std::sync::{Arc, mpsc};

use tokio::sync::{oneshot, watch};
use vessel_app_view::desktop::window::{SAVE_DELAY, WindowState};

use super::{DesktopError, store::Store};

const QUEUE_CAPACITY: usize = 64;

enum Command {
    Changed {
        state: WindowState,
        publication: Publication,
    },
    Write {
        state: WindowState,
        publication: Publication,
        mode: WriteMode,
        done: oneshot::Sender<Result<(), DesktopError>>,
    },
}

#[derive(Debug, Clone)]
struct Publication(Arc<()>);

#[derive(Clone, Copy)]
enum WriteMode {
    Checkpoint,
    Final,
}

/// Process-wide handle; only the primary window supplies remembered bounds.
#[derive(Debug)]
pub struct WindowWriter {
    commands: mpsc::SyncSender<Command>,
    publication: watch::Sender<Publication>,
}

impl WindowWriter {
    /// Start one owner for this file, away from the rendering thread.
    ///
    /// # Errors
    /// Thread startup failures preserve the original OS cause.
    pub fn start(store: Store) -> Result<Self, DesktopError> {
        let (commands, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let initial = Publication(Arc::new(()));
        let (publication, _initial) = watch::channel(initial.clone());
        let _worker = std::thread::Builder::new()
            .name("vessel-window-state".into())
            .spawn(move || run(&store, receiver, initial))?;
        Ok(Self {
            commands,
            publication,
        })
    }

    /// Backpressure is explicit; a full queue never blocks an input handler.
    ///
    /// # Errors
    /// A full or closed worker requires the caller to report failure.
    pub fn changed(&self, state: WindowState) -> Result<(), DesktopError> {
        let publication = Publication(Arc::new(()));
        self.commands
            .try_send(Command::Changed {
                state,
                publication: publication.clone(),
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => DesktopError::Busy,
                mpsc::TrySendError::Disconnected(_) => DesktopError::Closed,
            })?;
        let _previous = self.publication.send_replace(publication);
        Ok(())
    }

    /// A final save is queued after all previous writes and acknowledged once done.
    ///
    /// # Errors
    /// Returns file errors or a worker that stopped before acknowledging the save.
    pub fn finalize(
        &self,
        state: WindowState,
    ) -> impl Future<Output = Result<(), DesktopError>> + 'static + use<> {
        self.write(state, WriteMode::Final)
    }

    /// Persist the latest bounds before quit/restart without closing the worker.
    /// A refused document guard or failed restart can keep this app running.
    /// If geometry arrived after capture, persist that newer actor state; the
    /// lifecycle owner still needs its immediate UI currentness fence.
    ///
    /// # Errors
    /// File refusal and worker/channel failures retain the same typed cause.
    pub fn flush(
        &self,
        state: WindowState,
    ) -> impl Future<Output = Result<(), DesktopError>> + 'static + use<> {
        self.write(state, WriteMode::Checkpoint)
    }

    fn write(
        &self,
        state: WindowState,
        mode: WriteMode,
    ) -> impl Future<Output = Result<(), DesktopError>> + 'static + use<> {
        let commands = self.commands.clone();
        // Capture now, before the future's blocking sender is scheduled. A
        // later accepted geometry must not be erased by this older checkpoint.
        let publication = self.publication.borrow().clone();
        async move {
            let (done, wait) = oneshot::channel();
            // Sending can wait for capacity, so it runs on a dedicated blocking
            // thread rather than inside GPUI's foreground future.
            let _sender = std::thread::Builder::new()
                .name("vessel-window-flush".into())
                .spawn(move || {
                    drop(commands.send(Command::Write {
                        state,
                        publication,
                        mode,
                        done,
                    }));
                })?;
            wait.await.map_err(|_closed| DesktopError::Closed)?
        }
    }
}

fn run(store: &Store, receiver: mpsc::Receiver<Command>, mut publication: Publication) {
    let mut pending = None;
    let mut latest = None;
    loop {
        let next = if pending.is_some() {
            receiver.recv_timeout(SAVE_DELAY)
        } else {
            receiver
                .recv()
                .map_err(|_closed| mpsc::RecvTimeoutError::Disconnected)
        };
        match next {
            Ok(Command::Changed {
                state,
                publication: next,
            }) => {
                publication = next;
                latest = Some(state);
                pending = Some(state);
            }
            Ok(Command::Write {
                state,
                publication: captured,
                mode,
                done,
            }) => {
                let state = match mode {
                    WriteMode::Checkpoint if !Arc::ptr_eq(&publication.0, &captured.0) => {
                        latest.unwrap_or(state)
                    }
                    WriteMode::Checkpoint | WriteMode::Final => state,
                };
                pending = None;
                latest = Some(state);
                let saved = store.save_window(state);
                match mode {
                    WriteMode::Checkpoint => drop(done.send(saved)),
                    WriteMode::Final => {
                        // A completed final receipt also promises the queue is
                        // closed, independent of which thread wakes first.
                        drop(receiver);
                        drop(done.send(saved));
                        break;
                    }
                }
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

    struct Temp(std::path::PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _cleanup = std::fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn delayed_checkpoint_sender_cannot_erase_geometry_accepted_after_capture() {
        use vessel_app_view::desktop::window::Presentation;
        let root = Temp(std::env::temp_dir().join(format!(
            "vessel-window-checkpoint-order-{}",
            vessel_proto::SessionId::random()
        )));
        let store = Store::new(root.0.clone()).unwrap();
        let writer = WindowWriter::start(store.clone()).unwrap();
        let older = WindowState::default();
        let newer =
            WindowState::new(Some((40.0, 50.0)), 900.0, 700.0, Presentation::Windowed).unwrap();
        writer.changed(older).unwrap();
        // The future captures A now, but its dedicated sender starts only
        // when it is polled below, after Changed(B) has entered the queue.
        let captured = writer.flush(older);
        writer.changed(newer).unwrap();
        captured.await.unwrap();
        assert_eq!(store.load_window().unwrap(), Some(newer));
        let finalizing = writer.finalize(newer);
        writer.changed(older).unwrap();
        finalizing.await.unwrap();
        assert_eq!(
            store.load_window().unwrap(),
            Some(newer),
            "explicit final snapshot wins"
        );
        assert!(matches!(writer.changed(older), Err(DesktopError::Closed)));
    }

    #[tokio::test]
    async fn checkpoint_acknowledges_state_and_keeps_the_writer_alive_after_refused_quit() {
        use vessel_app_view::desktop::window::Presentation;
        let root = Temp(std::env::temp_dir().join(format!(
            "vessel-window-checkpoint-{}",
            vessel_proto::SessionId::random()
        )));
        let store = Store::new(root.0.clone()).unwrap();
        let writer = WindowWriter::start(store.clone()).unwrap();
        let checkpoint =
            WindowState::new(Some((20.0, 30.0)), 800.0, 600.0, Presentation::Windowed).unwrap();
        writer.changed(WindowState::default()).unwrap();
        writer.flush(checkpoint).await.unwrap();
        assert_eq!(store.load_window().unwrap(), Some(checkpoint));
        // A document veto or failed updater restart leaves normal writes usable.
        let later =
            WindowState::new(Some((40.0, 50.0)), 900.0, 700.0, Presentation::Maximized).unwrap();
        writer.changed(later).unwrap();
        writer.flush(later).await.unwrap();
        assert_eq!(store.load_window().unwrap(), Some(later));
        writer.finalize(later).await.unwrap();
        assert!(matches!(
            writer.changed(checkpoint),
            Err(DesktopError::Closed)
        ));
    }

    #[tokio::test]
    async fn failed_checkpoint_does_not_close_the_queue_or_overwrite_an_unreadable_document() {
        let root = Temp(std::env::temp_dir().join(format!(
            "vessel-window-checkpoint-error-{}",
            vessel_proto::SessionId::random()
        )));
        let blocked = root.0.join("window-state.json");
        std::fs::create_dir_all(&blocked).unwrap();
        let store = Store::new(root.0.clone()).unwrap();
        let writer = WindowWriter::start(store.clone()).unwrap();
        assert!(writer.flush(WindowState::default()).await.is_err());
        assert!(blocked.is_dir());
        std::fs::remove_dir(&blocked).unwrap();
        writer.flush(WindowState::default()).await.unwrap();
        assert_eq!(store.load_window().unwrap(), Some(WindowState::default()));
        writer.finalize(WindowState::default()).await.unwrap();
    }

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
