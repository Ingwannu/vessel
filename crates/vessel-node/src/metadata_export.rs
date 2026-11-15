//! Preserve current session metadata before any export peer could mask it.
//! The assignment/peer rule is shared with reply projection, not copied here.

use crate::error::ServiceError;
use vessel_proto::{
    ExportSession, Forwarded, ProtocolVersion, Reply, Request as _, SessionInfo, TransferDetail,
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

/// A newer source/home does not prove that the next importing peer is newer.
/// Inspect the returned manifest before that connection masks its reply.
pub(crate) fn forwarded(value: &Forwarded, peer: ProtocolVersion) -> Result<(), ServiceError> {
    forwarded_at(value, vessel_proto::lifecycle_version::SINCE, peer)
}

fn forwarded_at(
    value: &Forwarded,
    assigned: Option<ProtocolVersion>,
    peer: ProtocolVersion,
) -> Result<(), ServiceError> {
    let mut reply = value.reply.as_ref();
    while let Reply::Forward(value) | Reply::Relay(value) | Reply::Deliver(value) = reply {
        reply = value.reply.as_ref();
    }
    if let Reply::ExportSession(manifest) = reply
        && let TransferDetail::Session { info, .. } = &manifest.detail
    {
        check_at(info, assigned, peer)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
