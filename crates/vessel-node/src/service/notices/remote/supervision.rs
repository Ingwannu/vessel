//! Finite local lifetime after following: a remote close is best-effort only.
use std::{future::Future, time::Duration};
use tokio_util::sync::CancellationToken;
use vessel_proto::{Ack, ErrorCode, WireError};

/// Cleanup may not retain a local Following permit behind an unresponsive peer.
pub(super) const CLEANUP_BUDGET: Duration = Duration::from_secs(5);
/// Setup can wait for an actor handover, but cannot retain shutdown forever.
pub(super) const SETUP_BUDGET: Duration = Duration::from_secs(120);

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
    // Failure-notice delivery and close share one finite owner budget. A
    // blocked parent queue must not prevent this timer from ever starting.
    let finishing = async {
        after(end).await;
        match close.await {
            Ok(_ack) => Cleanup::Confirmed,
            Err(error) => Cleanup::Refused(error.code),
        }
    };
    tokio::time::timeout(CLEANUP_BUDGET, finishing)
        .await
        .unwrap_or(Cleanup::Uncertain)
}

pub(super) async fn setup<T, U, L, P, M, E>(
    lookup: L,
    prompt: M,
    parent_ended: E,
    stop: &CancellationToken,
) -> Result<(T, U), WireError>
where
    L: Future<Output = Result<T, WireError>>,
    P: Future<Output = Result<U, WireError>>,
    M: FnOnce() -> P,
    E: Future<Output = ()>,
{
    let work = async {
        let info = lookup.await?;
        let tracked = prompt().await?;
        Ok((info, tracked))
    };
    tokio::select! {
        biased;
        () = stop.cancelled() => Err(unknown_setup()),
        () = parent_ended => Err(unknown_setup()),
        result = tokio::time::timeout(SETUP_BUDGET, work) => {
            result.unwrap_or_else(|_late| Err(unknown_setup()))
        }
    }
}

fn unknown_setup() -> WireError {
    // TrackPrompt may already have been dispatched. No absent-effect claim,
    // fabricated receipt or automatic retry follows loss of its response.
    WireError::new(
        ErrorCode::Unavailable,
        "remote notification setup could not be confirmed; inspect the worker before retrying",
    )
}
