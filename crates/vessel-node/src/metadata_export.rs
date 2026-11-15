//! Refuse lossy export metadata and settle any returned transfer before refusal.
//! Visibility is shared with reply projection; transport stays in the envelope.

use crate::error::ServiceError;
use std::future::Future;
use tokio_util::task::TaskTracker;
use vessel_proto::{
    ExportSession, Forwarded, ProtocolVersion, Reply, Request as _, SessionInfo, TransferDetail,
    TransferId, WireError,
};
use vessel_rpc::CallError;

/// Local imports have no wire projection. A peer export checks the exact info
/// snapshot that the exporter will include, before provider history is read.
pub(crate) fn check(info: &SessionInfo, peer: Option<ProtocolVersion>) -> Result<(), ServiceError> {
    peer.map_or(Ok(()), |peer| {
        check_at(info, vessel_proto::lifecycle_version::SINCE, peer)
    })
}

fn check_at(
    info: &SessionInfo,
    assigned: Option<ProtocolVersion>,
    peer: ProtocolVersion,
) -> Result<(), ServiceError> {
    if (info.labels.is_empty() && info.listing.is_shown())
        || vessel_proto::lifecycle_version::visible(assigned, peer)
    {
        return Ok(());
    }
    let method = ExportSession::METHOD;
    let error = assigned.map_or(CallError::UnassignedProtocol { method }, |needs| {
        CallError::NotInProtocol {
            method,
            needs,
            negotiated: peer,
        }
    });
    Err(ServiceError::MetadataProtocol(error))
}

/// Check the returned manifest before peer projection and retain its close
/// independently of the requesting task when that peer cannot receive it.
pub(crate) async fn settle<F, C>(
    value: &Forwarded,
    peer: ProtocolVersion,
    tasks: &TaskTracker,
    close: C,
) -> Result<(), ServiceError>
where
    F: Future<Output = Result<(), WireError>> + Send + 'static,
    C: FnOnce(TransferId) -> F + Send + 'static,
{
    settle_at(
        value,
        vessel_proto::lifecycle_version::SINCE,
        peer,
        tasks,
        close,
    )
    .await
}

async fn settle_at<F, C>(
    value: &Forwarded,
    assigned: Option<ProtocolVersion>,
    peer: ProtocolVersion,
    tasks: &TaskTracker,
    close: C,
) -> Result<(), ServiceError>
where
    F: Future<Output = Result<(), WireError>> + Send + 'static,
    C: FnOnce(TransferId) -> F + Send + 'static,
{
    let Some((info, transfer)) = manifest(value) else {
        return Ok(());
    };
    let Err(refusal) = check_at(info, assigned, peer) else {
        return Ok(());
    };
    // Dropping the join handle detaches, rather than aborts, this tracked task.
    // Expiry is already the source's uncertain-close policy; do not retry export.
    let cleanup = tasks.spawn(async move {
        match tokio::time::timeout(crate::transfers::IDLE, close(transfer)).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%transfer, code = ?error.code,
                "refused export cleanup was not confirmed; source expiry remains"),
            Err(error) => tracing::warn!(%transfer, %error,
                "refused export cleanup timed out; source expiry remains"),
        }
    });
    if let Err(error) = cleanup.await {
        tracing::warn!(%transfer, %error, "refused export cleanup task failed; source expiry remains");
    }
    Err(refusal)
}

fn manifest(value: &Forwarded) -> Option<(&SessionInfo, TransferId)> {
    let mut reply = value.reply.as_ref();
    while let Reply::Forward(value) | Reply::Relay(value) | Reply::Deliver(value) = reply {
        reply = value.reply.as_ref();
    }
    if let Reply::ExportSession(manifest) = reply
        && let TransferDetail::Session { info, .. } = &manifest.detail
    {
        Some((info, manifest.transfer))
    } else {
        None
    }
}

#[cfg(test)]
fn forwarded_at(
    value: &Forwarded,
    assigned: Option<ProtocolVersion>,
    peer: ProtocolVersion,
) -> Result<(), ServiceError> {
    manifest(value).map_or(Ok(()), |(info, _)| check_at(info, assigned, peer))
}

#[cfg(test)]
mod cleanup_tests;
#[cfg(test)]
pub(crate) mod tests;
