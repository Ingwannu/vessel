//! One configured registration generation, owned by the existing Relays state.
//!
//! [Decision Log]
//! - Purpose: stopping registrations must not release their capacity early.
//! - Constraint: callbacks and established data connections are independent.
//! - Choice: keep the recipe local and one generation tracker until completion;
//!   settings wait on a clone, so caller cancellation cannot lose ownership.
//! - Why: the existing node tracker can track the same registration futures.
//! - Cost: an uncertain stop remains disabling; enable refuses overlap.
use super::{Serving, Tracking};
use std::sync::Arc;
use tokio::sync::watch;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::{
    RelayToken, RouteUrl,
    relay_settings::{ConfiguredRelays, RelayChange, RelayHealth, RelaySettingsError},
};
use vessel_relay::{LinkState, NodeLink, TlsConfig};
use vessel_secure::NodeKeypair;

pub(crate) const MAX_CONFIGURED_RELAYS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Advertised {
    Hidden,
    Visible,
}

/// The single owner's route decision; no independent authority or route cache.
#[derive(Debug, Clone)]
pub(crate) struct Advertisement(watch::Receiver<Advertised>);
impl Advertisement {
    pub(crate) fn includes_configured(&self) -> bool {
        self.0.has_changed().is_ok() && *self.0.borrow() == Advertised::Visible
    }
}

struct Recipe {
    routes: Vec<RouteUrl>,
    token: RelayToken,
    tls: TlsConfig,
}
impl std::fmt::Debug for Recipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recipe")
            .field("route_count", &self.routes.len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Active,
    Stopping,
}

struct Generation {
    phase: Phase,
    stop: CancellationToken,
    tasks: TaskTracker,
    links: Vec<(RouteUrl, watch::Receiver<LinkState>)>,
}

impl std::fmt::Debug for Generation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Generation")
            .field("phase", &self.phase)
            .field("registration_count", &self.links.len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(super) struct Configured {
    recipe: Option<Recipe>,
    desired: RelayChange,
    generation: Option<Generation>,
    advertised: watch::Sender<Advertised>,
}
impl Default for Configured {
    fn default() -> Self {
        let (advertised, _receiver) = watch::channel(Advertised::Hidden);
        Self {
            recipe: None,
            desired: RelayChange::Disabled,
            generation: None,
            advertised,
        }
    }
}
impl Configured {
    pub(super) fn configure(
        &mut self,
        routes: &[RouteUrl],
        token: &RelayToken,
        tls: &TlsConfig,
    ) -> Result<(), RelaySettingsError> {
        if self.recipe.is_some() {
            return Err(RelaySettingsError::Busy);
        }
        // Startup validates the cap before token/TLS I/O and before any task.
        // Refuse here as well: private callers cannot bypass that invariant.
        if routes.is_empty() || routes.len() > MAX_CONFIGURED_RELAYS {
            return Err(RelaySettingsError::NotConfigured);
        }
        self.recipe = Some(Recipe {
            routes: routes.to_vec(),
            token: token.clone(),
            tls: tls.clone(),
        });
        self.initial(None);
        Ok(())
    }

    pub(super) fn initial(&mut self, saved: Option<RelayChange>) {
        self.desired = saved.unwrap_or(RelayChange::Enabled);
        self.publish();
    }

    fn publish(&self) {
        let advertised = if self.recipe.is_some() && self.desired == RelayChange::Enabled {
            Advertised::Visible
        } else {
            Advertised::Hidden
        };
        let _previous = self.advertised.send_replace(advertised);
    }

    pub(super) fn advertisement(&self) -> Advertisement {
        Advertisement(self.advertised.subscribe())
    }

    fn reap(&mut self) {
        if self.generation.as_ref().is_some_and(|generation| {
            generation.phase == Phase::Stopping && generation.tasks.is_empty()
        }) {
            self.generation = None;
        }
    }

    pub(super) fn validate(&mut self, change: RelayChange) -> Result<(), RelaySettingsError> {
        self.reap();
        if change == RelayChange::Unknown {
            return Err(RelaySettingsError::Unknown);
        }
        if self.recipe.is_none() {
            return Err(RelaySettingsError::NotConfigured);
        }
        if change == RelayChange::Enabled
            && self
                .generation
                .as_ref()
                .is_some_and(|generation| generation.phase == Phase::Stopping)
        {
            return Err(RelaySettingsError::Busy);
        }
        Ok(())
    }

    pub(super) fn start(
        &mut self,
        serving: &Serving,
        keypair: &Arc<NodeKeypair>,
        node_stop: &CancellationToken,
    ) {
        if self.desired != RelayChange::Enabled || node_stop.is_cancelled() {
            return;
        }
        if self
            .generation
            .as_ref()
            .is_some_and(|generation| !generation.tasks.is_empty())
        {
            return;
        }
        let Some(recipe) = &self.recipe else {
            return;
        };
        let stop = node_stop.child_token();
        let tasks = TaskTracker::new();
        let mut links = Vec::with_capacity(recipe.routes.len());
        for route in &recipe.routes {
            let link = NodeLink::new(
                route.clone(),
                Arc::clone(keypair),
                recipe.token.clone(),
                recipe.tls.clone(),
            );
            links.push((route.clone(), link.state()));
            serving.run(
                link,
                stop.child_token(),
                Tracking::Configured(tasks.clone()),
            );
        }
        let _was_open = tasks.close();
        self.generation = Some(Generation {
            phase: Phase::Active,
            stop,
            tasks,
            links,
        });
    }

    /// Cancellation only starts the stop; the state retains its completion owner.
    pub(super) fn disable(&mut self) -> Option<TaskTracker> {
        self.desired = RelayChange::Disabled;
        self.publish();
        let generation = self.generation.as_mut()?;
        generation.phase = Phase::Stopping;
        generation.stop.cancel();
        Some(generation.tasks.clone())
    }

    pub(super) fn enable(&mut self) {
        self.desired = RelayChange::Enabled;
        self.publish();
    }

    pub(super) fn states(&self) -> Vec<(RouteUrl, watch::Receiver<LinkState>)> {
        self.generation
            .as_ref()
            .map_or_else(Vec::new, |generation| generation.links.clone())
    }

    pub(super) fn snapshot(&mut self) -> ConfiguredRelays {
        self.reap();
        if self.recipe.is_none() {
            return ConfiguredRelays::Unconfigured;
        }
        if self
            .generation
            .as_ref()
            .is_some_and(|generation| generation.phase == Phase::Stopping)
        {
            return ConfiguredRelays::Disabling;
        }
        match self.desired {
            RelayChange::Disabled => ConfiguredRelays::Disabled,
            RelayChange::Unknown => ConfiguredRelays::Unknown,
            RelayChange::Enabled => ConfiguredRelays::Enabled {
                health: self
                    .generation
                    .as_ref()
                    .map_or(RelayHealth::Connecting, |generation| {
                        health(&generation.links)
                    }),
            },
        }
    }
}

fn health(links: &[(RouteUrl, watch::Receiver<LinkState>)]) -> RelayHealth {
    let mut connecting = false;
    let mut retrying = false;
    let mut unknown = false;
    for (_route, state) in links {
        let current = state.borrow();
        if state.has_changed().is_err() && !matches!(*current, LinkState::Stopped) {
            unknown = true;
            continue;
        }
        match &*current {
            LinkState::Registered => return RelayHealth::Online,
            LinkState::Connecting => connecting = true,
            LinkState::Refused(_) => retrying = true,
            LinkState::Stopped => {}
        }
    }
    if connecting {
        RelayHealth::Connecting
    } else if retrying {
        RelayHealth::Retrying
    } else if unknown {
        RelayHealth::Unknown
    } else {
        RelayHealth::Stopped
    }
}

#[cfg(test)]
mod tests;
