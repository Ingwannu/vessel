//! Finite local lifetime after following: a remote close is best-effort only.
use std::{future::Future, time::Duration};
use tokio_util::sync::CancellationToken;
use vessel_proto::{Ack, ErrorCode, WireError};

/// Cleanup may not retain a local Following permit behind an unresponsive peer.
pub(super) const CLEANUP_BUDGET: Duration = Duration::from_secs(5);

pub(super) enum End {
    Stopped,
    ParentEnded,
    Followed(Result<(), WireError>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Cleanup {
    Confirmed,
    Refused(ErrorCode),
    Uncertain,
}

pub(super) async fn run<F, E, A, N, C>(
    follow: F,
    parent_ended: E,
    stop: CancellationToken,
    after: A,
    close: C,
) -> Cleanup
where
    F: Future<Output = Result<(), WireError>>,
    E: Future<Output = ()>,
    A: FnOnce(End) -> N,
    N: Future<Output = ()>,
    C: Future<Output = Result<Ack, WireError>>,
{
    let end = tokio::select! {
        biased;
        () = stop.cancelled() => End::Stopped,
        () = parent_ended => End::ParentEnded,
        result = follow => End::Followed(result),
    };
    after(end).await;
    // Timeout is explicitly uncertain. The remote receipt's existing finite
    // expiry owns eventual reclamation; there is no automatic cleanup retry.
    match tokio::time::timeout(CLEANUP_BUDGET, close).await {
        Ok(Ok(_ack)) => Cleanup::Confirmed,
        Ok(Err(error)) => Cleanup::Refused(error.code),
        Err(_late) => Cleanup::Uncertain,
    }
}
