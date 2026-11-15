//! One service-port choice, staged until the allocator's plan owner is wired.
//! This module neither reserves global ports nor decides when a helper runs.

use std::collections::HashSet;
use std::ffi::OsString;
use std::io;
use std::net::Ipv4Addr;

use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use vessel_proto::WorkspaceId;
use vessel_proto::scripts::{ScriptEntry, ScriptKind};
use vessel_proto::workspace::{InvalidServicePort, PortRange, ServicePort, ServicePorts};

use super::port_script::{self, PortScriptError};
use crate::workspace::ScriptRoot;

/// Caller-supplied entropy makes the range's starting offset deterministic in tests.
#[derive(Clone, Copy)]
pub(super) struct RangeOffset(u16);

impl RangeOffset {
    /// Every seed is valid; the chosen range reduces it to an in-range offset.
    pub(super) const fn from_seed(seed: [u8; 2]) -> Self {
        Self(u16::from_le_bytes(seed))
    }
}

/// Borrow the approved declaration and allocation choices from the same parse.
pub(super) struct Allocation<'a> {
    /// Exact logical owner supplied by the checked registry caller, never inferred.
    pub(super) workspace: WorkspaceId,
    /// The approved checkout supplied by the workspace registry.
    pub(super) root: &'a ScriptRoot,
    /// The service owner, including its validated optional explicit port.
    pub(super) entry: &'a ScriptEntry,
    /// The same file's allocation block, with helper precedence over range.
    pub(super) options: Option<&'a ServicePorts>,
    /// All ports already planned in this place, including explicit declarations.
    pub(super) reserved: &'a HashSet<ServicePort>,
    /// Other dynamic owners, excluding this service's previous reservation.
    pub(super) dynamic: &'a HashSet<ServicePort>,
    /// Only a range uses this caller-supplied starting offset.
    pub(super) offset: RangeOffset,
    /// The binary's credential inventory, reused by the direct helper.
    pub(super) removed: &'a [OsString],
    /// Cancels this choice and any helper child it owns.
    pub(super) cancel: &'a CancellationToken,
}

/// A refused choice keeps its actual decoder, helper or operating-system cause.
#[derive(Debug, thiserror::Error)]
pub(super) enum AllocationError {
    #[error("only a service can request an allocated port")]
    NotService,
    #[error("service port {0} is already reserved in this place")]
    Duplicate(ServicePort),
    #[error("the port script returned already-reserved port {0}")]
    HelperReserved(ServicePort),
    #[error("the port script failed")]
    Program(#[source] PortScriptError),
    #[error("no available service port in configured range {range}")]
    Exhausted {
        range: PortRange,
        #[source]
        last: Option<io::Error>,
    },
    #[error("a validated service port range could not be traversed")]
    RangeArithmetic,
    #[error("the allocated socket did not report a TCP port")]
    Socket(#[source] io::Error),
    #[error("the allocated socket reported an invalid service port")]
    Port(#[source] InvalidServicePort),
    #[error("service-port allocation was cancelled")]
    Cancelled,
}

/// Apply explicit port, program, range, then ephemeral precedence without PATH.
pub(super) async fn allocate(request: Allocation<'_>) -> Result<ServicePort, AllocationError> {
    tokio::select! {
        biased;
        () = request.cancel.cancelled() => Err(AllocationError::Cancelled),
        port = choose(&request) => port,
    }
}

async fn choose(request: &Allocation<'_>) -> Result<ServicePort, AllocationError> {
    if request.entry.kind() != ScriptKind::Service {
        return Err(AllocationError::NotService);
    }
    if let Some(port) = request.entry.port() {
        return if request.reserved.contains(&port) {
            Err(AllocationError::Duplicate(port))
        } else {
            Ok(port)
        };
    }
    let unavailable: HashSet<_> = request.reserved.union(request.dynamic).copied().collect();
    if let Some(program) = request.options.and_then(ServicePorts::port_script) {
        let port = port_script::invoke(
            request.workspace,
            request.root,
            &request.entry.name,
            program,
            request.removed,
            request.cancel,
        )
        .await
        .map_err(AllocationError::Program)?;
        return if unavailable.contains(&port) {
            Err(AllocationError::HelperReserved(port))
        } else {
            Ok(port)
        };
    }
    if let Some(range) = request.options.and_then(ServicePorts::range) {
        return from_range(range, request.offset, &unavailable, request.cancel).await;
    }
    let socket = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(AllocationError::Socket)?;
    let number = socket.local_addr().map_err(AllocationError::Socket)?.port();
    // The plan owner retries a kernel choice that collides with its dynamic
    // reservations, just as Paseo's resolveServicePort does; this owns no plan.
    ServicePort::try_from(number).map_err(AllocationError::Port)
}

async fn from_range(
    range: PortRange,
    offset: RangeOffset,
    reserved: &HashSet<ServicePort>,
    cancel: &CancellationToken,
) -> Result<ServicePort, AllocationError> {
    let mut last = None;
    for number in candidates(range, offset)? {
        if cancel.is_cancelled() {
            return Err(AllocationError::Cancelled);
        }
        let port = ServicePort::try_from(number).map_err(AllocationError::Port)?;
        if reserved.contains(&port) {
            continue;
        }
        match TcpListener::bind((Ipv4Addr::LOCALHOST, number)).await {
            Ok(socket) => {
                drop(socket);
                return Ok(port);
            }
            Err(error) => last = Some(error),
        }
    }
    Err(AllocationError::Exhausted { range, last })
}

/// Walk each endpoint once, wrapping only after the requested starting offset.
fn candidates(
    range: PortRange,
    offset: RangeOffset,
) -> Result<impl Iterator<Item = u16>, AllocationError> {
    let start = range.start().get();
    let end = range.end().get();
    let width = end
        .checked_sub(start)
        .and_then(|width| width.checked_add(1))
        .ok_or(AllocationError::RangeArithmetic)?;
    let first = offset
        .0
        .checked_rem(width)
        .and_then(|offset| start.checked_add(offset))
        .ok_or(AllocationError::RangeArithmetic)?;
    Ok((first..=end).chain(start..first))
}

#[cfg(test)]
mod tests;
