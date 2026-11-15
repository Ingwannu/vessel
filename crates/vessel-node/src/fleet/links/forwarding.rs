//! Bind reply provenance to the actual authenticated connection, never id text.
use super::{CallError, Links, NodeClient, NodeId, PoisonError, WireError, hop_error, unavailable};
use vessel_proto::Forwarded;
use vessel_rpc::Unary;

impl Links {
    pub(super) fn home_client(&self) -> Result<NodeClient, WireError> {
        self.home
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or_else(|| {
                unavailable(
                    "this node is not connected to its fleet's home right now \
                     (`vessel fleet status`)"
                        .to_owned(),
                )
            })
    }

    pub(crate) async fn forward_home<R: Unary<Response = Forwarded>>(
        &self,
        request: R,
    ) -> Result<Forwarded, WireError> {
        let client = self.home_client()?;
        let mut reply = client
            .call(request)
            .await
            .map_err(|error| hop_error(client.node().id, error))?;
        reply.received_from(client.protocol());
        Ok(reply)
    }

    pub(crate) async fn call_member<R: Unary<Response = Forwarded>>(
        &self,
        node: NodeId,
        request: R,
    ) -> Result<Forwarded, WireError> {
        let client = self.member_client(node).await?;
        let called = client.call(request).await;
        if matches!(called, Err(CallError::Disconnected)) {
            drop(self.members.lock().await.remove(&node));
        }
        let mut reply = called.map_err(|error| hop_error(node, error))?;
        reply.received_from(client.protocol());
        Ok(reply)
    }
}
