//! Cancel-safe actual proxy drain, distinct from a caller's close deadline.
use super::ProxyError;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Only successful completion of the real listener/stream/preparation scope creates this.
#[derive(Debug)]
pub(crate) struct ProxyStopped(());

enum State {
    Waiting(JoinHandle<Result<(), ProxyError>>),
    Stopped,
    Unconfirmed,
}
/// The future process owner retains this alongside its profile and capacity.
/// Dropping a caller's wait cannot detach the join from this retained owner.
pub(crate) struct ClosingProxy {
    state: State,
    cancel: CancellationToken,
}
impl std::fmt::Debug for ClosingProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosingBrowserProxy")
            .finish_non_exhaustive()
    }
}
impl ClosingProxy {
    pub(super) fn new(
        actor: Option<JoinHandle<Result<(), ProxyError>>>,
        cancel: CancellationToken,
    ) -> Self {
        cancel.cancel();
        Self {
            state: actor.map_or(State::Unconfirmed, State::Waiting),
            cancel,
        }
    }
    /// No internal deadline: the retained process owner waits through actual cleanup.
    /// A separate caller can stop waiting without releasing that owner's resources.
    pub(crate) async fn wait(&mut self) -> Result<ProxyStopped, ProxyError> {
        let result = match &mut self.state {
            State::Stopped => return Ok(ProxyStopped(())),
            State::Unconfirmed => return Err(ProxyError::Unconfirmed),
            State::Waiting(actor) => match actor.await {
                Ok(result) => result,
                Err(error) => Err(ProxyError::Task(error)),
            },
        };
        match result {
            Ok(()) => {
                self.state = State::Stopped;
                Ok(ProxyStopped(()))
            }
            Err(error) => {
                self.state = State::Unconfirmed;
                Err(error)
            }
        }
    }
}
impl Drop for ClosingProxy {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
