//! Explicit account/command inputs for catalogue work. This layer starts only
//! admitted work; it never discovers at node startup or reads a login file.

use super::cache::{Observation, Work};
use crate::profiles::launch::McpSource;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use vessel_agent::{AgentSetup, Driver, ProviderProbeError};
use vessel_harness::{
    HarnessEnv, Lookup,
    providers::{
        CommandPrefix, ProviderLaunchError, ProviderProtocol, ToolEnforcement, provider_launch,
        provider_transport,
    },
};
use vessel_proto::{
    ConfigValue, ResolvedMcpServers,
    providers::{Arguments, CatalogueScope, DiscoveryFailure, DiscoveryPhase, UnavailableReason},
};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub(super) struct Producer {
    lookup: Lookup,
    account: HarnessEnv,
    probe_dir: PathBuf,
    secrets: Arc<dyn McpSource>,
}

impl Producer {
    pub(super) fn new(
        lookup: Lookup,
        account: HarnessEnv,
        probe_dir: PathBuf,
        secrets: Arc<dyn McpSource>,
    ) -> Self {
        Self {
            lookup,
            account,
            probe_dir,
            secrets,
        }
    }

    pub(super) async fn observe(&self, work: Work, cancel: CancellationToken) -> Observation {
        let Some(transport) = provider_transport(work.definition.kind) else {
            return Observation::Unavailable(UnavailableReason::DriverMissing);
        };
        let driver = match transport.protocol {
            ProviderProtocol::Acp => Driver::Acp,
            ProviderProtocol::Direct(vessel_harness::DirectProtocol::CodexAppServer) => {
                Driver::CodexAppServer
            }
            ProviderProtocol::Direct(vessel_harness::DirectProtocol::ClaudeStream) => {
                Driver::ClaudeStream
            }
            ProviderProtocol::Direct(vessel_harness::DirectProtocol::OpencodeHttp) => {
                Driver::OpencodeHttp
            }
        };
        if !driver.built() {
            return Observation::Unavailable(UnavailableReason::DriverMissing);
        }
        let mut secrets = BTreeMap::new();
        for value in work.definition.launch.env.values().values() {
            if let ConfigValue::Secret(name) = value {
                let Some(value) = self.secrets.secret(name) else {
                    return Observation::Failed(DiscoveryFailure::ConfigurationUnavailable);
                };
                let _previous = secrets.insert(name.clone(), value);
            }
        }
        let (lookup, account, definition) = (
            self.lookup.clone(),
            self.account.clone(),
            Arc::clone(&work.definition),
        );
        let cwd = match &work.scope {
            CatalogueScope::Global => self.probe_dir.clone(),
            CatalogueScope::Workspace(directory) => directory.path().to_path_buf(),
        };
        let launched = tokio::task::spawn_blocking(move || {
            let default = transport
                .program
                .map(|program| -> Result<CommandPrefix, ProviderLaunchError> {
                    Ok(CommandPrefix {
                        program: program
                            .parse()
                            .map_err(|_invalid| ProviderLaunchError::NoDefault)?,
                        args: Arguments::default(),
                    })
                })
                .transpose()?;
            let launch = provider_launch(
                default.as_ref(),
                &definition.launch,
                &lookup,
                &account,
                &secrets,
                |_| None,
            )?;
            let (mut command, _denied) = launch.into_parts(ToolEnforcement::Unsupported)?;
            // Account homes are explicit values, never read from the daemon's
            // process; configured environment values keep final precedence.
            if let Some(variable) = transport.home
                && !definition
                    .launch
                    .env
                    .values()
                    .keys()
                    .any(|name| name.as_str().eq_ignore_ascii_case(variable))
                && let Some(home) = account.var(variable)
            {
                command = command.env(variable, home);
            }
            Ok::<_, ProviderLaunchError>(command.args(transport.arguments).cwd(cwd))
        })
        .await;
        let command = match launched {
            Ok(Ok(command)) => command,
            Ok(Err(ProviderLaunchError::Unavailable)) => {
                return Observation::Unavailable(UnavailableReason::NotInstalled);
            }
            Ok(Err(_unsupported)) => {
                return Observation::Failed(DiscoveryFailure::ConfigurationUnavailable);
            }
            Err(_lost) => return Observation::Failed(DiscoveryFailure::ProbeFailed),
        };
        let cwd = command
            .cwd
            .clone()
            .unwrap_or_else(|| self.probe_dir.clone());
        let setup = AgentSetup {
            cwd,
            model: None,
            resume: None,
            fork: None,
            options: Vec::new(),
            mcp_servers: ResolvedMcpServers::new(),
            handshake_timeout: HANDSHAKE_TIMEOUT,
            questions: false,
            private_dir: None,
        };
        let observed = tokio::select! {
            () = cancel.cancelled() => return Observation::Failed(DiscoveryFailure::Superseded),
            observed = vessel_agent::probe_provider_with(driver, &command, setup, &work.definition) => observed,
        };
        match observed {
            Ok(observed) => Observation::Ready {
                catalogue: observed.catalogue,
                version: observed.version,
            },
            Err(
                ProviderProbeError::Codex(vessel_agent::CodexCatalogueError::TimedOut)
                | ProviderProbeError::Probe(vessel_agent::ProbeError::TimedOut { .. }),
            ) => Observation::Failed(DiscoveryFailure::TimedOut {
                phase: DiscoveryPhase::Catalogue,
            }),
            Err(
                ProviderProbeError::Catalogue(_)
                | ProviderProbeError::Codex(vessel_agent::CodexCatalogueError::Invalid),
            ) => Observation::Failed(DiscoveryFailure::InvalidCatalogue),
            Err(_) => Observation::Failed(DiscoveryFailure::ProbeFailed),
        }
    }
}

#[cfg(test)]
mod tests;
