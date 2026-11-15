//! A bounded persistence phase never repeats node finalization on a changed receipt.

use gpui::{App, AsyncApp};
use std::future::{Future as _, poll_fn};
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, Instant};
use vessel_app_link::desktop::DesktopError;

const MAX_STATE_ATTEMPTS: usize = 4;
const STATE_DEADLINE: Duration = Duration::from_secs(10);
const RECONCILE_YIELD: Duration = Duration::from_millis(1);

#[derive(Debug)]
pub(crate) enum StateFailure {
    Deadline,
    Changed,
    Write(Arc<DesktopError>),
}

impl std::fmt::Display for StateFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Deadline => "desktop state did not settle before its deadline",
            Self::Changed => "desktop state changed during every bounded save attempt",
            Self::Write(_) => "desktop state could not be saved",
        })
    }
}

impl std::error::Error for StateFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write(error) => Some(error.as_ref()),
            Self::Deadline | Self::Changed => None,
        }
    }
}

/// `effect` runs on the App executor immediately after the final receipt fence.
/// It must combine final document admission and exit/restart without an await.
pub(crate) async fn flush_then<T>(
    cx: &AsyncApp,
    effect: impl FnOnce(&mut App) -> T,
) -> Result<T, StateFailure> {
    let started = Instant::now();
    let mut effect = Some(effect);
    for _attempt in 0..MAX_STATE_ATTEMPTS {
        let remaining = STATE_DEADLINE.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Err(StateFailure::Deadline);
        }
        let flush = cx.update(crate::desktop::flush_state);
        let timer = cx.background_executor().timer(remaining);
        let mut flush = std::pin::pin!(flush);
        let mut timer = std::pin::pin!(timer);
        let receipt: crate::desktop::StateReceipt = poll_fn(|cx| {
            if started.elapsed() >= STATE_DEADLINE {
                return Poll::Ready(Err(StateFailure::Deadline));
            }
            if let Poll::Ready(result) = flush.as_mut().poll(cx) {
                return Poll::Ready(result.map_err(|error| StateFailure::Write(Arc::new(error))));
            }
            if timer.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Err(StateFailure::Deadline));
            }
            Poll::Pending
        })
        .await?;
        let admitted = cx.update(|cx| {
            use crate::desktop::StateReadiness;
            match crate::desktop::flush_current(&receipt, cx) {
                StateReadiness::Changed => Ok(None),
                StateReadiness::Failed(error) => Err(StateFailure::Write(error)),
                StateReadiness::Saved | StateReadiness::LaunchOnly => {
                    let Some(effect) = effect.take() else {
                        return Err(StateFailure::Changed);
                    };
                    Ok(Some(effect(cx)))
                }
            }
        })?;
        if let Some(result) = admitted {
            return Ok(result);
        }
        // Let accepted preference observations reconcile on the UI executor.
        // A retry takes a new receipt and never reruns owned-node shutdown.
        cx.background_executor().timer(RECONCILE_YIELD).await;
    }
    Err(StateFailure::Changed)
}
