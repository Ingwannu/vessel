//! Push subscriptions are bound to this paired device, not a wire-supplied
//! identifier. Delegation never borrows a fleet member's push identity.
use super::{Connection, NodeService};
use crate::principal::Principal;
use std::sync::Arc;
use vessel_proto::push::{
    GetPushKey, PushKey, PushLease, ReportPushPresence, SubscribePush, UnsubscribePush,
};
use vessel_proto::{Ack, DeviceId, ErrorCode, WireError};

fn device(connection: &Connection) -> Result<DeviceId, WireError> {
    match connection.principal() {
        Principal::Device { id, .. } => Ok(*id),
        Principal::Owner | Principal::Agent { .. } | Principal::Delegated { .. } => {
            Err(WireError::new(
                ErrorCode::Forbidden,
                "push subscriptions belong to a paired device; connect using that device's pairing before subscribing",
            ))
        }
    }
}
impl NodeService {
    pub(crate) async fn get_push_key(
        &self,
        connection: &Connection,
        _: GetPushKey,
    ) -> Result<PushKey, WireError> {
        let _device = device(connection)?;
        self.push.key().await.map_err(|error| error.wire())
    }
    pub(crate) async fn subscribe_push(
        &self,
        connection: &Connection,
        request: SubscribePush,
    ) -> Result<PushLease, WireError> {
        self.push
            .subscribe(device(connection)?, request.subscription)
            .await
            .map_err(|error| error.wire())
    }
    pub(crate) async fn unsubscribe_push(
        &self,
        connection: &Connection,
        _: UnsubscribePush,
    ) -> Result<Ack, WireError> {
        self.push
            .unsubscribe(device(connection)?)
            .await
            .map(|()| Ack)
            .map_err(|error| error.wire())
    }
    /// Watch transport closure once per registered connection, using the
    /// existing subscription tracker so shutdown also releases presence.
    pub(crate) async fn report_push_presence(
        &self,
        connection: &Connection,
        report: ReportPushPresence,
    ) -> Result<Ack, WireError> {
        let ack = self.record_push_presence(connection, report)?;
        if let Principal::Device { id, .. } = connection.principal() {
            self.push.renew(*id).await.map_err(|error| error.wire())?;
        }
        Ok(ack)
    }
    fn record_push_presence(
        &self,
        connection: &Connection,
        report: ReportPushPresence,
    ) -> Result<Ack, WireError> {
        match connection.principal() {
            Principal::Owner | Principal::Device { .. } => {}
            Principal::Agent { .. } | Principal::Delegated { .. } => {
                return Err(WireError::new(
                    ErrorCode::Forbidden,
                    "only a person's connected app reports activity; open a paired app to change notification presence",
                ));
            }
        }
        if self.push.present(connection.presence_id, report)? {
            let push = Arc::clone(&self.push);
            let id = connection.presence_id;
            let outbound = connection.outbound.clone();
            drop(self.forwarders.spawn(async move {
                outbound.closed().await;
                push.forget(id);
            }));
        }
        Ok(Ack)
    }
}
