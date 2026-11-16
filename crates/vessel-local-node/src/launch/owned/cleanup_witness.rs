//! Unit-test observation from the SAME checked cleanup owner, never a waiter.

use super::{LaunchError, ProcessExit};
use std::cell::RefCell;
use tokio::sync::oneshot;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Settlement {
    Reaped(ProcessExit),
    Unconfirmed,
}

struct Observers {
    settled: Option<oneshot::Sender<Settlement>>,
    dropped: Option<oneshot::Sender<()>>,
}

tokio::task_local! {
    static OBSERVERS: RefCell<Observers>;
}

/// Task-scoped only: parallel launches cannot receive each other's witness.
pub(crate) async fn observe<F: Future>(
    settled: oneshot::Sender<Settlement>,
    dropped: oneshot::Sender<()>,
    launch: F,
) -> F::Output {
    OBSERVERS
        .scope(
            RefCell::new(Observers {
                settled: Some(settled),
                dropped: Some(dropped),
            }),
            launch,
        )
        .await
}

pub(super) fn settled(result: &Result<ProcessExit, LaunchError>) {
    let _unobserved = OBSERVERS.try_with(|observers| {
        if let Some(sender) = observers.borrow_mut().settled.take() {
            let fact = match result {
                Ok(exit) => Settlement::Reaped(*exit),
                Err(_unconfirmed) => Settlement::Unconfirmed,
            };
            let _receiver_gone = sender.send(fact);
        }
    });
}

/// Declared after Child in Owned: its drop/release path ran before this emits.
/// On unconfirmed running-child cleanup this orders the hard-kill fallback.
/// This is only owner-drop observation, never an independent reap receipt.
#[derive(Default)]
pub(crate) struct Dropped(Option<oneshot::Sender<()>>);

pub(super) fn dropped() -> Dropped {
    Dropped(
        OBSERVERS
            .try_with(|observers| observers.borrow_mut().dropped.take())
            .ok()
            .flatten(),
    )
}

impl Drop for Dropped {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _receiver_gone = sender.send(());
        }
    }
}
