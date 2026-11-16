//! Prompt-free Codex catalogue projection using the driver's common read probe.
//! Model/config interpretation and bounded catalogue policy remain here.

pub use super::read_probe::ProbeError as CatalogueProbeError;

use super::{
    catalogue_data::{self, ConfigResponse, Defaults, ModelPage},
    mode::Mode,
    read_probe::{self, ReadRequest, Transaction},
};
use crate::{AgentSetup, ProviderObservation};
use vessel_harness::providers::{ProviderDefinition, catalogue_from_options};
use vessel_proc::CommandSpec;
use vessel_proto::{agents::AgentVersion, providers::MAX_CATALOGUE_ENTRIES};

const MAX_PAGES: usize = 64;

#[derive(serde::Serialize)]
struct Models<'a> {
    cursor: Option<&'a str>,
}
impl ReadRequest for Models<'_> {
    type Response = ModelPage;
    const METHOD: &'static str = "model/list";
}

#[derive(serde::Serialize)]
struct SavedConfig {}
impl ReadRequest for SavedConfig {
    type Response = ConfigResponse;
    const METHOD: &'static str = "getUserSavedConfig";
}

#[derive(serde::Serialize)]
struct Config {}
impl ReadRequest for Config {
    type Response = ConfigResponse;
    const METHOD: &'static str = "config/read";
}

pub(crate) async fn probe(
    command: &CommandSpec,
    setup: &AgentSetup,
    definition: &ProviderDefinition,
) -> Result<ProviderObservation, CatalogueProbeError> {
    let command = command.clone().cwd(&setup.cwd);
    read_probe::run(&command, setup.handshake_timeout, |transaction| {
        transaction.read_catalogue(definition)
    })
    .await
}

impl Transaction {
    async fn read_catalogue(
        mut self,
        definition: &ProviderDefinition,
    ) -> Result<ProviderObservation, CatalogueProbeError> {
        let initialized = self.initialize().await?;
        let version = initialized.user_agent.as_deref().and_then(version);
        let mut native = Vec::new();
        let mut budget = catalogue_data::MetadataBudget::default();
        let mut cursor: Option<String> = None;
        for _page in 0..MAX_PAGES {
            let page = self
                .required(&Models {
                    cursor: cursor.as_deref(),
                })
                .await?;
            if native.len().saturating_add(page.data.len()) > MAX_CATALOGUE_ENTRIES {
                return Err(CatalogueProbeError::Invalid);
            }
            budget.admit(&page)?;
            native.extend(page.data);
            match page.next_cursor {
                None => {
                    cursor = None;
                    break;
                }
                Some(next) if next.is_empty() || cursor.as_ref() == Some(&next) => {
                    return Err(CatalogueProbeError::Invalid);
                }
                Some(next) => cursor = Some(next),
            }
        }
        if cursor.is_some() {
            return Err(CatalogueProbeError::Invalid);
        }
        // Older Codex versions may lack either config method. Only an explicit
        // method-not-found is optional; malformed successful data is a failure.
        let saved: Option<ConfigResponse> = self.request(&SavedConfig {}).await?;
        let config: Option<ConfigResponse> = self.request(&Config {}).await?;
        let defaults = saved
            .map_or_else(Defaults::default, |saved| saved.config)
            .fallback(config.map_or_else(Defaults::default, |config| config.config));
        let mut catalogue = catalogue_from_options(definition, &[Mode::DEFAULT.option()])
            .map_err(|_invalid| CatalogueProbeError::Invalid)?;
        catalogue.models = catalogue_data::models(native, &defaults)?;
        Ok(ProviderObservation { catalogue, version })
    }
}

fn version(user_agent: &str) -> Option<AgentVersion> {
    let word = user_agent.split_whitespace().next()?;
    let text = word.split_once('/').map_or(word, |(_, text)| text);
    text.parse().ok()
}
