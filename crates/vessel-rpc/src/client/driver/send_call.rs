//! A caller's existing oneshot closure fences requests until transport admission.
//!
//! Keep the waiter local while capacity/readiness waits are cancellable. Installing
//! it before start_send preserves reply ordering without phantom registrations.

use futures_util::{Sink, SinkExt as _};
use std::future::poll_fn;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use vessel_proto::{Call, ClientFrame, RequestId};

use super::{DriverError, Registration, Waiter};

pub(super) enum Transfer {
    Skipped,
    Sent,
    Closed,
}

pub(super) async fn send<K, E>(
    sink: &mut K,
    register: &mpsc::Sender<Registration>,
    closed: &CancellationToken,
    id: RequestId,
    call: Call,
    mut waiter: Waiter,
) -> Result<Transfer, DriverError<E>>
where
    K: Sink<String, Error = E> + Unpin,
{
    // This check does not depend on the caller successfully queuing Cancel.
    if waiter.is_closed() {
        return Ok(Transfer::Skipped);
    }
    let text =
        serde_json::to_string(&ClientFrame::Request { id, call }).map_err(DriverError::Encode)?;
    let registration = tokio::select! { biased;
        () = closed.cancelled() => return Ok(Transfer::Closed),
        () = waiter.closed() => return Ok(Transfer::Skipped),
        permit = register.reserve() => match permit {
            Ok(permit) => permit,
            Err(_reader_gone) => return Ok(Transfer::Closed),
        },
    };
    tokio::select! { biased;
        () = closed.cancelled() => return Ok(Transfer::Closed),
        () = waiter.closed() => return Ok(Transfer::Skipped),
        ready = poll_fn(|cx| sink.poll_ready_unpin(cx)) => ready.map_err(DriverError::Transport)?,
    }
    // One synchronous admission: no await between the final closure check,
    // reply registration and handing the frame to the ready sink. A concurrent
    // drop cannot revoke bytes this admission has already committed.
    if closed.is_cancelled() {
        return Ok(Transfer::Closed);
    }
    if waiter.is_closed() {
        return Ok(Transfer::Skipped);
    }
    registration.send(Registration::Wait(id, waiter));
    sink.start_send_unpin(text)
        .map_err(DriverError::Transport)?;
    Ok(Transfer::Sent)
}

#[cfg(test)]
mod tests;
