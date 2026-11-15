//! Account-free Web Push to this authenticated paired device (ADR 0041).
use crate::NodeClient;
use vessel_proto::push::{
    GetPushKey, PushKey, PushLease, PushSubscription, ReportPushPresence, SubscribePush,
    UnsubscribePush,
};
use vessel_rpc::CallError;
impl NodeClient {
    /// Node's public VAPID identity for the browser subscription.
    ///
    /// # Errors
    /// `forbidden` except for paired devices, `NotInProtocol` on an older
    /// node, or a private-key setup/transport failure.
    pub async fn push_key(&self) -> Result<PushKey, CallError> {
        self.call(GetPushKey).await
    }
    /// Replace or renew this device's subscription; never another device's.
    ///
    /// # Errors
    /// `invalid_request` for invalid keys/private endpoints, `forbidden`
    /// without paired read access, `conflict` at the 64-device limit, or
    /// `unavailable` when DNS/setup cannot finish. Older nodes refuse locally.
    pub async fn subscribe_push(
        &self,
        subscription: PushSubscription,
    ) -> Result<PushLease, CallError> {
        self.call(SubscribePush { subscription }).await
    }
    /// Remove this device's subscription; succeeds when it had none.
    ///
    /// # Errors
    /// `forbidden` except for paired read devices, or a store/transport failure.
    pub async fn unsubscribe_push(&self) -> Result<(), CallError> {
        self.call(UnsubscribePush).await.map(|_ack| ())
    }
    /// Report real input and focus on this connection; the node stamps time.
    ///
    /// # Errors
    /// `forbidden` for agents/delegation, `unavailable` at the bounded
    /// presence limit, or a protocol/transport failure.
    pub async fn report_push_presence(&self, report: ReportPushPresence) -> Result<(), CallError> {
        self.call(report).await.map(|_ack| ())
    }
}
