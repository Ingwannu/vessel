//! A node's plugin catalog and plugin method calls (ADR 0115, "Catalog and
//! invoke", 2026-11-20).
//!
//! [Decision Log]
//! - Purpose: Paseo's daemon answers `plugin.catalog.get` with every
//!   running plugin's client part and runs `plugin.rpc.invoke` against a
//!   running plugin's registered methods
//!   (`packages/server/src/server/session.ts`); the page and the app draw a
//!   machine's plugin themes and commands from the first and run them with
//!   the second.
//! - Existing pieces: the reviewed `ClientCatalog` admits the documents
//!   (owner, duplicates, counts and bytes); the supervisor's runtime state
//!   holds a helper's current declarations and its handle calls only
//!   methods that generation registered (`CallError::Unregistered`).
//! - Choice: the catalog is built from the rows under the plain lock: a
//!   client-only installation's document from its start, a helper's from
//!   its runtime state. Not running (stopped, failed, plugins off) is not
//!   in the catalog. It waits for the start-up settle as the list does, so
//!   a client reading at node start does not see an empty catalog first.
//! - Invoke checks that the installation runs a helper whose current run is
//!   the one the caller's catalog named (ADR 0079: an action carries its
//!   captured generation, checked again at dispatch); the actor checks the
//!   method against that run's declarations and the payload budget. The
//!   helper receives `{input, context}`. Failures are safe classes: the
//!   helper's own error text is never in the reply (ADR 0079); what a
//!   helper writes to stderr is in its log.
//! - Cost: a catalog over the whole-node budget is refused as a whole
//!   rather than trimmed, so a client never draws half of a plugin set.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use vessel_plugin::{CallError, RuntimeState};
use vessel_proto::RawJson;
use vessel_proto::plugins::{
    ClientCatalog, ClientDocument, ClientInstallation, HelperCall, HelperPayload, InvokeContext,
    InvokePlugin, PluginAddress, PluginCatalog, RpcName,
};

use super::{PluginError, Plugins, Run};

impl Plugins {
    /// The running installations' declarations, at the current generation.
    pub(crate) async fn catalog(&self) -> Result<PluginCatalog, PluginError> {
        self.settled().await;
        let generation = *self.inner.generation.borrow();
        let installations = {
            let state = self.state();
            if state.switch {
                state
                    .rows
                    .values()
                    .filter_map(|row| entry(&row.run))
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };
        let required = vessel_proto::plugins::PLUGIN_CATALOG_PROTOCOL
            .unwrap_or(vessel_proto::PROTOCOL_VERSION);
        let installations = installations
            .into_iter()
            .map(|(address, document)| {
                ClientInstallation::new(address, required, (*document).clone())
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| PluginError::Catalog(error.to_string()))?;
        let catalog = ClientCatalog::new(self.inner.node, installations)
            .map_err(|error| PluginError::Catalog(error.to_string()))?;
        Ok(PluginCatalog {
            generation,
            catalog,
        })
    }

    /// Call the request's method on its plugin's running helper.
    pub(crate) async fn invoke(&self, request: InvokePlugin) -> Result<RawJson, PluginError> {
        let InvokePlugin {
            plugin,
            generation,
            method,
            input,
            context,
        } = request;
        let plugin = &plugin;
        let (handle, address) = {
            let state = self.state();
            let row = state
                .rows
                .get(plugin)
                .ok_or_else(|| PluginError::NotInstalled(plugin.clone()))?;
            match &row.run {
                Run::Helper(supervisor) => {
                    let handle = supervisor.handle();
                    let address = match &*handle.subscribe().borrow() {
                        RuntimeState::Running { address, .. } => Some(address.clone()),
                        RuntimeState::Starting | RuntimeState::Stopped => None,
                    };
                    (handle, address)
                }
                Run::Client { .. } => return Err(PluginError::NoMethods(plugin.clone())),
                Run::Stopped => return Err(PluginError::NotRunning(plugin.clone())),
            }
        };
        let address = address.ok_or_else(|| PluginError::NotRunning(plugin.clone()))?;
        if address.generation != generation {
            return Err(PluginError::Call {
                id: plugin.clone(),
                failure: CallFailure::Changed,
            });
        }
        let too_large = || PluginError::Call {
            id: plugin.clone(),
            failure: CallFailure::TooLarge,
        };
        let envelope = RawJson::from_serialize(&Envelope {
            input: &input,
            context: &context,
        })
        .map_err(|_unencodable| too_large())?;
        let input = HelperPayload::try_from(envelope).map_err(|_over| too_large())?;
        // The node's stop cancels a call in flight; a caller that goes
        // away leaves it to the actor's own deadline.
        let cancel: CancellationToken = self.inner.stop.child_token();
        let output = handle
            .call(
                address,
                HelperCall::Invoke {
                    method: method.clone(),
                    input,
                },
                &cancel,
            )
            .await
            .map_err(|error| PluginError::Call {
                id: plugin.clone(),
                failure: CallFailure::from_runtime(&error, method),
            })?;
        Ok(RawJson::from(output))
    }
}

/// What a helper's method receives.
#[derive(serde::Serialize)]
struct Envelope<'a> {
    input: &'a RawJson,
    context: &'a InvokeContext,
}

/// A running installation's catalog entry.
fn entry(run: &Run) -> Option<(PluginAddress, Arc<ClientDocument>)> {
    match run {
        Run::Client { address, document } => Some((address.clone(), Arc::clone(document))),
        Run::Helper(supervisor) => match &*supervisor.handle().subscribe().borrow() {
            RuntimeState::Running {
                address, client, ..
            } => Some((address.clone(), Arc::clone(client))),
            RuntimeState::Starting | RuntimeState::Stopped => None,
        },
        Run::Stopped => None,
    }
}

/// Why a method call failed, in classes safe to show any administrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallFailure {
    /// The running generation does not declare it.
    Undeclared(RpcName),
    /// The installation ran again since the caller's catalog.
    Changed,
    /// The input is over the helper payload budget.
    TooLarge,
    /// The helper is busy, restarting, stopping or did not answer in time.
    Unavailable,
    /// The plugin refused the call.
    Refused,
    /// The plugin's method failed; its own words are in its log.
    Failed,
}

impl CallFailure {
    fn from_runtime(error: &CallError, method: RpcName) -> Self {
        match error {
            CallError::Unregistered => Self::Undeclared(method),
            CallError::Stale => Self::Changed,
            CallError::Failed(vessel_proto::plugins::HelperFailure::Refused) => Self::Refused,
            CallError::Failed(vessel_proto::plugins::HelperFailure::Callback) => Self::Failed,
            CallError::NotReady
            | CallError::Busy
            | CallError::Cancelled
            | CallError::Timeout
            | CallError::Closed
            | CallError::Failed(vessel_proto::plugins::HelperFailure::Cancelled) => {
                Self::Unavailable
            }
        }
    }
}

impl std::fmt::Display for CallFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Undeclared(method) => write!(f, "it declares no method {method}"),
            Self::Changed => f.write_str("it changed since its catalog was read; read it again"),
            Self::TooLarge => f.write_str("the input is too large"),
            Self::Unavailable => {
                f.write_str("it is busy, restarting or stopping; try again in a moment")
            }
            Self::Refused => f.write_str("it refused the call"),
            Self::Failed => f.write_str("its method failed; its log may say why"),
        }
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
