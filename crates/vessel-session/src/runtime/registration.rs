//! Tests can pause the real resumed-create continuation after lookup and before
//! spawn/registration, forcing the competing executor's conversation claim.

use std::sync::{Mutex, PoisonError};

use tokio::sync::oneshot;

#[derive(Debug, Default)]
pub(super) struct RegistrationGate(Mutex<Option<oneshot::Sender<Registration>>>);

#[derive(Debug)]
pub(super) struct Registration(Option<oneshot::Sender<()>>);

impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(release) = self.0.take() {
            let _gone = release.send(());
        }
    }
}

impl RegistrationGate {
    pub(super) fn arm(&self) -> oneshot::Receiver<Registration> {
        let (entered, arriving) = oneshot::channel();
        let previous = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(entered);
        assert!(
            previous.is_none(),
            "one real creation may be held at a time"
        );
        arriving
    }

    pub(super) async fn wait(&self) {
        let entered = self.0.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(entered) = entered {
            let (release, waiting) = oneshot::channel();
            if entered.send(Registration(Some(release))).is_ok() {
                let _released = waiting.await;
            }
        }
    }
}
