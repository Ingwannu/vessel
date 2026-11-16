//! Private construction keeps successful data tied to a real actor observation.
use crate::DeliveryGuard;
use vessel_app_view::workspace::registry_projection::{
    RegistryData, RegistryReadFailure, RegistryReadKey,
};

/// Complete guarded observation, or a correlated local failure.
#[derive(Debug)]
pub struct Response {
    key: RegistryReadKey,
    guard: Option<DeliveryGuard>,
    result: Result<RegistryData, RegistryReadFailure>,
    #[cfg(not(target_arch = "wasm32"))]
    capacity: Option<std::sync::Arc<tokio::sync::OwnedSemaphorePermit>>,
}
impl Response {
    pub(super) fn refused(key: RegistryReadKey) -> Self {
        Self::failure(key, RegistryReadFailure::Refused)
    }
    pub(super) fn failure(key: RegistryReadKey, why: RegistryReadFailure) -> Self {
        Self {
            key,
            guard: None,
            result: Err(why),
            #[cfg(not(target_arch = "wasm32"))]
            capacity: None,
        }
    }
    /// Apply only after the original weak observation is validated. The receiver
    /// retains that same guard and validates it again at every subsequent query.
    /// The queued-response permit remains held until this callback returns.
    pub fn receive<T>(
        self,
        apply: impl FnOnce(
            RegistryReadKey,
            Option<DeliveryGuard>,
            Result<RegistryData, RegistryReadFailure>,
        ) -> T,
    ) -> T {
        #[cfg(not(target_arch = "wasm32"))]
        let _capacity = self.capacity;
        let result = if self
            .guard
            .as_ref()
            .is_some_and(|guard| guard.validate(self.key.context()).is_err())
        {
            Err(RegistryReadFailure::Offline)
        } else {
            self.result
        };
        apply(self.key, self.guard, result)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn publish(
    selected: &crate::connections::SelectedConnection,
    events: &tokio::sync::mpsc::Sender<crate::Event>,
    key: RegistryReadKey,
    guard: DeliveryGuard,
    result: Result<RegistryData, RegistryReadFailure>,
    capacity: std::sync::Arc<tokio::sync::OwnedSemaphorePermit>,
) {
    const PUBLICATION_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
    let response = Response {
        key: key.clone(),
        guard: Some(guard),
        result,
        capacity: Some(std::sync::Arc::clone(&capacity)),
    };
    let published = tokio::time::timeout(
        PUBLICATION_DEADLINE,
        selected.send_guarded(events, crate::Event::RegistryRead(response)),
    )
    .await;
    match published {
        Ok(Ok(()) | Err(crate::connections::SendFailure::ReceiverClosed)) => return,
        Ok(Err(crate::connections::SendFailure::Refused(_))) | Err(_) => {}
    }
    // The bounded sender retains the call slot while a correlated refusal waits;
    // a dropping timeout here would strand the app's outstanding ticket.
    let mut response = Response::failure(key, RegistryReadFailure::Offline);
    response.capacity = Some(capacity);
    let _closed = events.send(crate::Event::RegistryRead(response)).await;
}
