//! Lifecycle receipts fence current UI state; quit policy remains with its owner.

use super::Desktop;
use gpui::App;
use std::sync::Arc;
use vessel_app_link::desktop::{
    DesktopError,
    preferences::{Checkpoint, Outcome},
};
use vessel_app_view::desktop::window::WindowState;

/// Storage acknowledgment is not admission: recheck with `flush_current` at exit.
#[derive(Debug)]
pub(crate) struct StateReceipt {
    preferences: Option<Checkpoint>,
    geometry: Option<GeometryReceipt>,
}

#[derive(Debug)]
struct GeometryReceipt {
    state: WindowState,
    publication: Arc<()>,
}

impl StateReceipt {
    /// Forced shutdown only logs this; its observer cannot veto or promise completion.
    pub(super) fn failure(&self) -> Option<&Arc<DesktopError>> {
        match self.preferences.as_ref()?.snapshot().outcome() {
            Outcome::Failed(error) => Some(error),
            Outcome::Saved | Outcome::LaunchOnly => None,
        }
    }
}

/// No await may follow an admitted result before quit/updater restart.
#[derive(Debug)]
pub(crate) enum StateReadiness {
    /// New geometry, publication or unreconciled UI requires another bounded attempt.
    Changed,
    /// Every writable owner is acknowledged and still current.
    Saved,
    /// At least one initial unreadable/unavailable store remains launch-only.
    LaunchOnly,
    /// The current preference write was explicitly refused.
    Failed(Arc<DesktopError>),
}

/// Capture both stores before awaiting, keeping their workers alive after a veto.
pub(crate) fn flush_state(
    cx: &App,
) -> impl Future<Output = Result<StateReceipt, DesktopError>> + 'static + use<> {
    let preferences = cx
        .try_global::<Desktop>()
        .map(|desktop| desktop.preferences.checkpoint());
    let geometry = cx.try_global::<Desktop>().and_then(|desktop| {
        desktop.writer.as_ref().map(|writer| {
            (
                GeometryReceipt {
                    state: desktop.latest,
                    publication: Arc::clone(&desktop.geometry_publication),
                },
                writer.flush(desktop.latest),
            )
        })
    });
    async move {
        let preferences = match preferences {
            Some(checkpoint) => Some(checkpoint.await?),
            None => None,
        };
        let geometry = match geometry {
            Some((state, flushed)) => {
                flushed.await?;
                Some(state)
            }
            None => None,
        };
        Ok(StateReceipt {
            preferences,
            geometry,
        })
    }
}

/// App-level only, after `Shell` borrows end. Existing live records are the sole
/// registry; the quit owner combines this fence with its final document guard.
pub(crate) fn flush_current(receipt: &StateReceipt, cx: &App) -> StateReadiness {
    let Some(desktop) = cx.try_global::<Desktop>() else {
        return if receipt.preferences.is_none() {
            StateReadiness::LaunchOnly
        } else {
            StateReadiness::Changed
        };
    };
    let Some(preferences) = &receipt.preferences else {
        return StateReadiness::Changed;
    };
    if !desktop.preferences.is_current(preferences)
        || receipt.geometry.as_ref().is_some_and(|geometry| {
            geometry.state != desktop.latest
                || !Arc::ptr_eq(&geometry.publication, &desktop.geometry_publication)
        })
        || receipt.geometry.is_some() != desktop.writer.is_some()
    {
        return StateReadiness::Changed;
    }
    let live = cx.windows();
    for record in cx
        .try_global::<crate::host_windows::Registry>()
        .into_iter()
        .flat_map(|registry| registry.records.iter())
        .filter(|record| live.contains(&record.window))
    {
        if let Some(shell) = record.shell.upgrade()
            && let Some(client) = &shell.read(cx).areas.desktop.preferences
            && !client.is_settled(preferences)
        {
            return StateReadiness::Changed;
        }
    }
    match preferences.snapshot().outcome() {
        Outcome::Failed(error) => StateReadiness::Failed(Arc::clone(error)),
        Outcome::LaunchOnly => StateReadiness::LaunchOnly,
        Outcome::Saved => {
            if receipt.geometry.is_some() {
                StateReadiness::Saved
            } else {
                StateReadiness::LaunchOnly
            }
        }
    }
}

#[cfg(test)]
mod tests;
