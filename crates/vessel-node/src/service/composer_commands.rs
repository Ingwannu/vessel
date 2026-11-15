//! Prompt-free draft command catalogs reuse workspace authorization and probe caching.
//! No prompt, durable session or extra MCP process is created by discovery.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use sha2::{Digest as _, Sha256};
use vessel_proto::composer_commands::{CommandCatalog, DraftCommandConfig, GetDraftCommands};
use vessel_proto::{ErrorCode, HarnessOptions, SessionId, StructuredSupport, WireError};
use vessel_session::ProbeChoices;

use super::{Connection, NodeService, wire};
use crate::models::Models;
use crate::probes::{Answer, ProbeKey};
use crate::readiness::HandshakeFailure;
use crate::workspace::WorkspaceError;

/// A cache key includes every selected draft field without retaining option text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DraftConfigHash([u8; 32]);

impl DraftConfigHash {
    pub(crate) fn of(config: &DraftCommandConfig) -> Result<Self, WireError> {
        let encoded = serde_json::to_vec(config).map_err(|error| {
            WireError::new(
                ErrorCode::Internal,
                format!("cannot encode command configuration: {error}"),
            )
        })?;
        Ok(Self(Sha256::digest(encoded).into()))
    }
}

/// A bounded configuration prevents a catalog query becoming an oversized launch.
const MAX_DRAFT_CHOICES: usize = 64;

fn validate(config: &DraftCommandConfig) -> Result<(), WireError> {
    if config.options.len() > MAX_DRAFT_CHOICES {
        return Err(WireError::new(
            ErrorCode::InvalidRequest,
            format!("a command draft accepts at most {MAX_DRAFT_CHOICES} option choices"),
        ));
    }
    let mut seen = HashSet::new();
    if config.options.iter().any(|choice| !seen.insert(&choice.id)) {
        return Err(WireError::new(
            ErrorCode::InvalidRequest,
            "a draft option can be selected only once",
        ));
    }
    Ok(())
}

fn profile_changed() -> WireError {
    WireError::new(
        ErrorCode::Unavailable,
        "the command profile changed during discovery",
    )
}

fn cacheable_profile(profile: &vessel_harness::Profile) -> Result<(), WireError> {
    if profile.model().is_some()
        || profile
            .added()
            .is_some_and(|agent| agent.secrets().next().is_some())
    {
        // The profile snapshot does not prove independently mutable routes or
        // fleet secrets. Keep those cases unavailable until their owner supplies
        // an actual immutable launch proof, rather than hashing credential values.
        return Err(WireError::new(
            ErrorCode::Unsupported,
            "command discovery needs an immutable route and secret configuration",
        ));
    }
    Ok(())
}

impl NodeService {
    pub(crate) async fn draft_commands(
        &self,
        _: &Connection,
        request: GetDraftCommands,
    ) -> Result<CommandCatalog, WireError> {
        let config = request.config;
        validate(&config)?;
        let cwd = self.catalog_directory(&config).await?;
        self.refuse_installing(&config.harness)
            .map_err(|error| wire(&error))?;
        let snapshot = self.profiles.current();
        let profile = self
            .resolve_in(&snapshot, &config.harness, config.profile.as_ref())
            .map_err(|error| wire(&error))?;
        cacheable_profile(&profile)?;
        let installed = self
            .harnesses()
            .await
            .into_iter()
            .find(|found| found.kind == config.harness)
            .ok_or_else(|| {
                WireError::new(
                    ErrorCode::HarnessNotInstalled,
                    "the draft provider is not installed",
                )
            })?;
        if let StructuredSupport::Unavailable { reason } = installed.structured {
            return Err(WireError::new(ErrorCode::Unsupported, reason));
        }
        let key = ProbeKey {
            harness: config.harness.clone(),
            profile: config.profile.clone(),
            draft: Some(DraftConfigHash::of(&config)?),
        };
        let answer = self
            .probes
            .answer_in_profile(key, installed.version, Arc::clone(&snapshot), async || {
                if !Arc::ptr_eq(&snapshot, &self.profiles.current()) {
                    return failed(profile_changed());
                }
                let answer = self.probe_command_draft(config, profile, cwd).await;
                if Arc::ptr_eq(&snapshot, &self.profiles.current()) {
                    answer
                } else {
                    failed(profile_changed())
                }
            })
            .await;
        // Cache hits and waiters must pass the same postcondition as new probes.
        if !Arc::ptr_eq(&snapshot, &self.profiles.current()) {
            return Err(profile_changed());
        }
        let _options = answer.options?;
        let commands = answer.commands.ok_or_else(|| {
            WireError::new(
                ErrorCode::Unsupported,
                "this provider did not report a prompt-free command catalog",
            )
        })?;
        Ok(CommandCatalog { commands })
    }

    async fn catalog_directory(&self, config: &DraftCommandConfig) -> Result<PathBuf, WireError> {
        let resolved = self
            .workspaces
            .resolve(config.root)
            .await
            .map_err(|error| error.wire())?;
        let directory = config.directory.clone();
        let checked = tokio::task::spawn_blocking(move || {
            let path = match directory {
                Some(directory) => resolved.root.existing(&directory)?,
                None => resolved.root.path().to_owned(),
            };
            if !path.is_dir() {
                return Err(WorkspaceError::Invalid(
                    "the draft selection must be a directory".to_owned(),
                ));
            }
            Ok(path)
        })
        .await
        .map_err(|error| {
            WireError::new(
                ErrorCode::Internal,
                format!("checking the draft directory stopped: {error}"),
            )
        })?;
        checked.map_err(|error| error.wire())
    }

    async fn probe_command_draft(
        &self,
        config: DraftCommandConfig,
        profile: vessel_harness::Profile,
        cwd: PathBuf,
    ) -> Answer {
        // Catalog cache hits need no permit; a new probe shares the existing
        // bounded agent-discovery budget with history listing.
        let Ok(_place) = self.listings.try_acquire() else {
            return failed(WireError::new(
                ErrorCode::Unavailable,
                "the node is already discovering as many provider catalogs as it can",
            ));
        };
        let probe = SessionId::random();
        let _model = ModelLease {
            models: &self.models,
            session: probe,
        };
        let proxy = match self.models.launch(Some(&profile), probe) {
            Ok(launched) => launched.map(|(_model, proxy)| proxy),
            Err(error) => return failed(wire(&error)),
        };
        let choices = ProbeChoices {
            model: config.model,
            options: config.options,
        };
        match self
            .sessions
            .probe_with_choices(config.harness, profile, proxy, cwd, choices)
            .await
        {
            Ok(probed) => Answer {
                commands: probed.commands,
                options: Ok(HarnessOptions {
                    options: probed.options,
                    probed_at: Utc::now(),
                    abilities: Some(probed.abilities),
                }),
                said: Ok(probed.introduction),
            },
            Err(error) => Answer {
                commands: None,
                said: Err(crate::readiness::failure_of(&error)),
                options: Err(wire(&error)),
            },
        }
    }
}

fn failed(error: WireError) -> Answer {
    Answer {
        options: Err(error),
        said: Err(HandshakeFailure::Spawn),
        commands: None,
    }
}

/// Cancellation must retire routed credentials even if the awaiting RPC is dropped.
struct ModelLease<'a> {
    models: &'a Models,
    session: SessionId,
}
impl Drop for ModelLease<'_> {
    fn drop(&mut self) {
        self.models.retire(self.session);
    }
}

#[cfg(test)]
mod tests;
