//! Canonical registry reads use one original physical admission through delivery.
//!
//! Four in-flight or queued observations share the view's finite read budget.
//! A response retains its permit through installation; queueing cannot release
//! capacity while keeping complete peer snapshots alive. Separate catalog reads
//! are not an atomic node snapshot: placement validation stays in the projection.

mod delivery;
#[cfg(not(target_arch = "wasm32"))]
mod read;
#[cfg(all(not(target_arch = "wasm32"), any(test, feature = "testing")))]
mod settlement;
pub use delivery::Response;
#[cfg(all(not(target_arch = "wasm32"), any(test, feature = "testing")))]
pub use settlement::{Settlement, SettlementError};

use vessel_app_view::workspace::registry_projection::RegistryReadKey;
use vessel_proto::registry::RegistryRevision;

/// Read correlation and cursor never replace the captured admission.
#[derive(Debug)]
pub struct Request {
    /// Exact original context and monotonically allocated UI read ticket.
    pub key: RegistryReadKey,
    /// Only a long-poll cursor, never a physical generation or mutation proof.
    pub changed_from: Option<RegistryRevision>,
}
impl Request {
    /// Settle an unsent command without manufacturing a successful observation.
    #[must_use]
    pub fn refused(self) -> Response {
        Response::refused(self.key)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub(crate) struct Calls(std::sync::Arc<tokio::sync::Semaphore>);
#[cfg(not(target_arch = "wasm32"))]
impl Default for Calls {
    fn default() -> Self {
        Self(std::sync::Arc::new(tokio::sync::Semaphore::new(
            vessel_app_view::workspace::registry_projection::MAX_REGISTRY_READS,
        )))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) async fn registry_read(&self, request: Request) {
        use vessel_app_view::workspace::registry_projection::RegistryReadFailure;
        let selected = match self.selected_connection(request.key.context()) {
            Ok(selected) => selected,
            Err(_retired) => {
                self.emit(crate::Event::RegistryRead(Response::failure(
                    request.key,
                    RegistryReadFailure::Offline,
                )))
                .await;
                return;
            }
        };
        let Ok(permit) =
            std::sync::Arc::clone(&self.workspace.registry_calls.0).try_acquire_owned()
        else {
            self.emit(crate::Event::RegistryRead(request.refused()))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            #[cfg(any(test, feature = "testing"))]
            let mut pending = settlement::Pending::default();
            execute(
                selected,
                events,
                request,
                permit,
                #[cfg(any(test, feature = "testing"))]
                &mut pending,
            )
            .await;
            // The awaited operation future and its selected connection/senders
            // are gone before normal settlement. A queued response owns its
            // independent capacity until the App receives it.
            #[cfg(any(test, feature = "testing"))]
            pending.complete();
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn execute(
    selected: crate::connections::SelectedConnection,
    events: tokio::sync::mpsc::Sender<crate::Event>,
    request: Request,
    permit: tokio::sync::OwnedSemaphorePermit,
    #[cfg(any(test, feature = "testing"))] pending: &mut settlement::Pending,
) {
    let permit = std::sync::Arc::new(permit);
    let guard = selected.delivery_guard();
    let result = read::bounded(&selected, request.changed_from).await;
    delivery::publish(
        &selected,
        &events,
        request.key,
        guard,
        result,
        permit,
        #[cfg(any(test, feature = "testing"))]
        pending,
    )
    .await;
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
