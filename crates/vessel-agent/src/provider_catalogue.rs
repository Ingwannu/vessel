//! Provider discovery reuses the existing finite, prompt-free probe. Native
//! protocol types stay here; harness policy maps its typed option observation.

use crate::{AgentSetup, Driver, ProbeError, probe_with};
use vessel_harness::providers::{
    CatalogueOptionsError, ProviderDefinition, catalogue_from_options,
};
use vessel_proc::CommandSpec;
use vessel_proto::{ResolvedMcpServers, agents::AgentVersion, providers::Catalogue};

/// One process observes models and modes together, then the probe closes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderObservation {
    /// Native choices, without manifest defaults presented as discovery.
    pub catalogue: Catalogue,
    /// Already-observed version, never a second implicit command.
    pub version: Option<AgentVersion>,
}

/// Private probe details retain their source but must not be returned as a
/// reader diagnostic message. The node maps them to safe failure classes.
#[derive(Debug, thiserror::Error)]
pub enum ProviderProbeError {
    /// Native Codex uses model/list rather than opening a throwaway thread.
    #[error("native provider catalogue probe failed")]
    Codex(#[from] crate::CodexCatalogueError),
    /// Process or protocol handshake failed.
    #[error("provider catalogue probe failed")]
    Probe(#[source] ProbeError),
    /// A successful handshake carried invalid or ambiguous choices.
    #[error("provider catalogue observation was invalid")]
    Catalogue(#[from] CatalogueOptionsError),
}

/// Observe a provider with no prompt, selected model, resumed conversation,
/// configured option change or MCP server. The caller supplies its explicit
/// authorized directory, account environment and compatible driver.
///
/// # Errors
/// Driver/probe failure or invalid catalogue refuses the whole observation.
pub async fn probe_provider_with(
    driver: Driver,
    command: &CommandSpec,
    setup: AgentSetup,
    definition: &ProviderDefinition,
) -> Result<ProviderObservation, ProviderProbeError> {
    if driver == Driver::Codex {
        return Ok(crate::codex::catalogue::probe(command, &setup, definition).await?);
    }
    let setup = AgentSetup {
        model: None,
        resume: None,
        fork: None,
        options: Vec::new(),
        mcp_servers: ResolvedMcpServers::new(),
        questions: false,
        private_dir: None,
        ..setup
    };
    let observation = probe_with(driver, command, setup)
        .await
        .map_err(ProviderProbeError::Probe)?;
    Ok(ProviderObservation {
        catalogue: catalogue_from_options(definition, &observation.options)?,
        version: observation.introduction.version,
    })
}
