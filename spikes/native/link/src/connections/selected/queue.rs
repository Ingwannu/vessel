//! Staged bounded event send; receiver-side validity remains independently required.
use super::{Refusal, SelectedConnection, Validity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SendFailure {
    Refused(Refusal),
    ReceiverClosed,
}
impl SelectedConnection {
    pub(crate) async fn send_guarded<T: Send>(
        &self,
        sender: &tokio::sync::mpsc::Sender<T>,
        event: T,
    ) -> Result<(), SendFailure> {
        self.active().map_err(SendFailure::Refused)?;
        let mut validity = self.lease.validity.clone();
        let sent = tokio::select! {
            biased;
            _retired = validity.wait_for(|state| *state == Validity::Retired) => Err(SendFailure::Refused(Refusal::Changed)),
            () = self.lease.client.closed() => Err(SendFailure::Refused(Refusal::Closed)),
            sent = sender.send(event) => sent.map_err(|_| SendFailure::ReceiverClosed),
        };
        self.fence(sent).await.map_err(SendFailure::Refused)?
    }
}
