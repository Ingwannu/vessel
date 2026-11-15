//! Prompt-free Codex catalogue transaction. One process initializes, reads model
//! pages and saved defaults, then stops. No thread, turn, sign-in or setter is
//! created. Existing session completion arms remain owned by their driver.

use super::{
    catalogue_data::{self, ConfigResponse, Defaults, ModelPage},
    mode::Mode,
    protocol::{ClientInfo, InitializeCapabilities, InitializeParams, InitializeResponse},
    wire::{self, Incoming, Outgoing},
};
use crate::{AgentSetup, ProviderObservation};
use std::time::Duration;
use tokio::{sync::mpsc, task::JoinSet};
use vessel_harness::providers::{ProviderDefinition, catalogue_from_options};
use vessel_proc::{CommandSpec, Signal};
use vessel_proto::providers::MAX_CATALOGUE_ENTRIES;

const INCOMING: usize = 16;
const MAX_PAGES: usize = 64;
const MAX_NOTIFICATIONS: usize = 256;
const STOP_GRACE: Duration = Duration::from_secs(2);
const METHOD_NOT_FOUND: i64 = -32601;

#[derive(Clone, Copy)]
enum Request {
    Initialize,
    Models,
    SavedConfig,
    Config,
}

impl Request {
    const fn method(self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Models => "model/list",
            Self::SavedConfig => "getUserSavedConfig",
            Self::Config => "config/read",
        }
    }
}

/// Safe failure classes; native error/log text never enters reader diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum CatalogueProbeError {
    /// The caller's finite probe deadline elapsed.
    #[error("Codex catalogue deadline elapsed")]
    TimedOut,
    /// Process setup failed; the source stays internal to the node.
    #[error("Codex catalogue process could not start")]
    Spawn(#[source] vessel_proc::SpawnError),
    /// Connection or a required request failed.
    #[error("Codex catalogue protocol failed")]
    Protocol,
    /// Native data violated bounded catalogue invariants.
    #[error("Codex catalogue observation was invalid")]
    Invalid,
}

pub(crate) async fn probe(
    command: &CommandSpec,
    setup: &AgentSetup,
    definition: &ProviderDefinition,
) -> Result<ProviderObservation, CatalogueProbeError> {
    let spawned =
        vessel_proc::spawn(&command.clone().cwd(&setup.cwd)).map_err(CatalogueProbeError::Spawn)?;
    let vessel_proc::Spawned {
        mut child,
        stdin,
        stdout,
        mut stderr,
    } = spawned;
    let out = Outgoing::start(stdin);
    let (send, incoming) = mpsc::channel(INCOMING);
    // JoinSet aborts both I/O tasks if the caller cancels. The child guard then
    // kills its entire group; a cancelled probe cannot detach readers/processes.
    let mut readers = JoinSet::new();
    let _stdout_task = readers.spawn(wire::read(stdout, send));
    let _stderr_task = readers.spawn(async move {
        let _discarded = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
    });
    let transaction = Transaction {
        out,
        incoming,
        next: 0,
    };
    let observed = tokio::time::timeout(setup.handshake_timeout, transaction.read(definition))
        .await
        .map_err(|_elapsed| CatalogueProbeError::TimedOut)
        .and_then(std::convert::identity);
    let _terminated = child.signal(Signal::Terminate);
    if tokio::time::timeout(STOP_GRACE, child.wait())
        .await
        .is_err()
    {
        let _killed = child.signal(Signal::Kill);
        let _reaped = tokio::time::timeout(STOP_GRACE, child.wait()).await;
    }
    readers.shutdown().await;
    observed
}

struct Transaction {
    out: Outgoing,
    incoming: mpsc::Receiver<Result<Incoming, wire::Ended>>,
    next: u64,
}

impl Transaction {
    async fn read(
        mut self,
        definition: &ProviderDefinition,
    ) -> Result<ProviderObservation, CatalogueProbeError> {
        let initialize = InitializeParams {
            client_info: ClientInfo {
                name: "vessel",
                title: "Vessel",
                version: env!("CARGO_PKG_VERSION"),
            },
            capabilities: InitializeCapabilities {
                experimental_api: true,
            },
        };
        let initialized: InitializeResponse =
            self.required(Request::Initialize, &initialize).await?;
        if !self.out.notify("initialized", &serde_json::json!({})).await {
            return Err(CatalogueProbeError::Protocol);
        }
        let version = initialized
            .user_agent
            .as_deref()
            .and_then(super::policy::version_of);
        let mut native = Vec::new();
        let mut budget = catalogue_data::MetadataBudget::default();
        let mut cursor: Option<String> = None;
        for _page in 0..MAX_PAGES {
            let params = serde_json::json!({ "cursor": cursor });
            let page: ModelPage = self.required(Request::Models, &params).await?;
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
        let saved: Option<ConfigResponse> = self.optional(Request::SavedConfig).await?;
        let config: Option<ConfigResponse> = self.optional(Request::Config).await?;
        let defaults = saved
            .map_or_else(Defaults::default, |saved| saved.config)
            .fallback(config.map_or_else(Defaults::default, |config| config.config));
        let policy = super::policy::CodexVersionPolicy::observed(version.as_ref());
        let mut catalogue =
            catalogue_from_options(definition, &[Mode::default_for(policy).option(policy)])
                .map_err(|_invalid| CatalogueProbeError::Invalid)?;
        catalogue.models = catalogue_data::models(native, &defaults)?;
        Ok(ProviderObservation { catalogue, version })
    }

    async fn required<T: serde::de::DeserializeOwned>(
        &mut self,
        method: Request,
        params: &impl serde::Serialize,
    ) -> Result<T, CatalogueProbeError> {
        let value = self
            .request(method, params)
            .await?
            .ok_or(CatalogueProbeError::Protocol)?;
        serde_json::from_value(value).map_err(|_invalid| CatalogueProbeError::Invalid)
    }

    async fn optional<T: serde::de::DeserializeOwned>(
        &mut self,
        method: Request,
    ) -> Result<Option<T>, CatalogueProbeError> {
        self.request(method, &serde_json::json!({}))
            .await?
            .map(|value| {
                serde_json::from_value(value).map_err(|_invalid| CatalogueProbeError::Invalid)
            })
            .transpose()
    }

    async fn request(
        &mut self,
        method: Request,
        params: &impl serde::Serialize,
    ) -> Result<Option<serde_json::Value>, CatalogueProbeError> {
        self.next = self
            .next
            .checked_add(1)
            .ok_or(CatalogueProbeError::Protocol)?;
        if !self.out.request(self.next, method.method(), params).await {
            return Err(CatalogueProbeError::Protocol);
        }
        for _message in 0..MAX_NOTIFICATIONS {
            match self.incoming.recv().await {
                Some(Ok(Incoming::Response { id, result })) if id == self.next => {
                    return Ok(Some(result));
                }
                Some(Ok(Incoming::Failure { id, error }))
                    if id == self.next && error.code == METHOD_NOT_FOUND =>
                {
                    return Ok(None);
                }
                Some(Ok(Incoming::Notification { .. })) => {}
                // A discovery process has no authority to ask for execution,
                // sign-in, tools or human answers. End it instead of acting.
                Some(
                    Ok(
                        Incoming::Response { .. }
                        | Incoming::Failure { .. }
                        | Incoming::Request { .. },
                    )
                    | Err(_),
                )
                | None => return Err(CatalogueProbeError::Protocol),
            }
        }
        Err(CatalogueProbeError::Protocol)
    }
}
