//! The home's dial to one member: one slot, shared by that member's callers.

use std::sync::{Arc, PoisonError};

use futures_util::FutureExt as _;
use vessel_client::secure::{SecureConnector, WsDialer};
use vessel_client::{Connector as _, DeviceCredential, NodeClient, Pinned, SecureTarget};
use vessel_proto::{ErrorCode, NodeId, WireError};

use super::files::FileError;
use super::{Links, MemberSlot, blocking, client_info, conflict, internal, unavailable};

impl Links {
    pub(super) async fn member_client(&self, node: NodeId) -> Result<NodeClient, WireError> {
        let slot = self.member_slot(node);
        // A tokio lock held across the dial below, on purpose: calls to the
        // same member wait for one dial and share its connection rather
        // than each opening one (a burst of calls would take every place
        // the relay keeps for that node), while calls to other members never
        // wait on it, so one unreachable member holds up only its own.
        let mut held = slot.lock().await;
        if let Some(client) = held.as_ref()
            && client.closed().now_or_never().is_none()
        {
            return Ok(client.clone());
        }
        // By the member device bound to `node`: exactly one, as grants are
        // checked when made. Two can only come from grants kept before
        // that check; neither is trusted.
        let mut bound = self.grants().await.into_iter().filter(|grant| grant.node == node);
        let grant = match (bound.next(), bound.next()) {
            (Some(grant), None) => grant,
            (None, _) => {
                return Err(WireError::new(
                    ErrorCode::NotFound,
                    format!(
                        "node {node} is not a member of this fleet that granted the home \
                         access (`vessel fleet nodes`)"
                    ),
                ));
            }
            (Some(first), Some(second)) => {
                return Err(conflict(format!(
                    "member devices {} and {} both claim node {node}; revoke the wrong one \
                     (`vessel fleet members`)",
                    first.member, second.member
                )));
            }
        };
        let (_json, credential_path) = self.grant_paths(grant.member);
        let credential = blocking(move || {
            DeviceCredential::read(&credential_path)
                .map_err(|error| FileError::io(&credential_path, std::io::Error::other(error)))
        })
        .await?;
        let target = SecureTarget {
            key: grant.key,
            relays: grant.relays.clone(),
            direct: grant.direct.clone(),
        };
        let dialer = WsDialer::for_routes(target.routes()).map_err(|error| internal(&error))?;
        let connector = SecureConnector::new(dialer, target, node, credential)
            .map_err(|error| internal(&error))?;
        let client = Pinned::new(connector, Some(node))
            .connect(client_info())
            .await
            .map_err(|error| unavailable(format!("cannot reach node {node}: {error}")))?;
        *held = Some(client.clone());
        Ok(client)
    }

    /// Member `node`'s slot, made on first use.
    pub(super) fn member_slot(&self, node: NodeId) -> MemberSlot {
        let mut members = self.members.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(members.entry(node).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(None))))
    }
}
