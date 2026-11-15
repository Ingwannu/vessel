//! Reader-safe snapshots and unregistered requests. Cached reads never probe an
//! executable or disclose launch configuration; active refresh is explicit.

use super::{Catalogue, CreateInput, Enabled, ProviderId, ProviderKind};
use crate::agents::{AgentLabel, AgentVersion};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Workspace catalogue path, never implicitly relative to a daemon's cwd.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "PathBuf", into = "PathBuf")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CatalogueDirectory(#[cfg_attr(feature = "ts", ts(type = "string"))] PathBuf);

impl CatalogueDirectory {
    /// Validate lexical input; the producer separately validates allowed access.
    ///
    /// # Errors
    /// Relative, non-text and control-containing paths are refused.
    pub fn new(path: PathBuf) -> Result<Self, InvalidCatalogueDirectory> {
        if !path.is_absolute()
            || path
                .to_str()
                .is_none_or(|text| text.chars().any(char::is_control))
        {
            return Err(InvalidCatalogueDirectory);
        }
        Ok(Self(path))
    }

    /// Target path; excluded from shareable diagnostic aggregates.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// Invalid catalogue target, without echoing private path text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("provider catalogue directory must be an absolute text path without controls")]
pub struct InvalidCatalogueDirectory;

impl TryFrom<PathBuf> for CatalogueDirectory {
    type Error = InvalidCatalogueDirectory;
    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl From<CatalogueDirectory> for PathBuf {
    fn from(directory: CatalogueDirectory) -> Self {
        directory.0
    }
}

/// Target is independent from provider identity and implementation family.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "scope", content = "directory", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum CatalogueScope {
    /// Host settings catalogue.
    Global,
    /// Explicit workspace catalogue.
    Workspace(CatalogueDirectory),
}

/// Public origin, without private plugin runtime registration details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum SnapshotSource {
    /// Manifest identity.
    Builtin,
    /// Configured alias, ACP or trusted plugin contribution.
    Custom,
}

/// Pending discovery stage, without raw error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DiscoveryPhase {
    /// Executable/support availability.
    Availability,
    /// Native version discovery.
    Version,
    /// One model/mode observation.
    Catalogue,
}

/// Safe failures; upstream logs, commands and endpoints stay in the producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DiscoveryFailure {
    /// A finite deadline interrupted an upstream activity.
    TimedOut {
        /// Activity whose deadline expired.
        phase: DiscoveryPhase,
    },
    /// Upstream failed without an exportable message.
    ProbeFailed,
    /// Result violated a typed catalogue invariant.
    InvalidCatalogue,
    /// Configuration changed while observation was in flight.
    Superseded,
    /// Owner is shutting down.
    Stopped,
}

/// Opt-in, implementation support and installation are separate facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum UnavailableReason {
    /// Explicit opt-out, including OMP's reference default.
    Disabled,
    /// No compatible driver is implemented.
    DriverMissing,
    /// Producer could not find a launcher.
    NotInstalled,
    /// Passive read had no observation and started no probe.
    NotChecked,
}

/// State carries exactly the values meaningful for that observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ProviderState {
    /// Refresh was admitted before its observation completed.
    Loading,
    /// Provider cannot currently be offered for creation.
    Unavailable {
        /// Opt-in/implementation/installation reason.
        reason: UnavailableReason,
    },
    /// Failed discovery never substitutes invented models or modes.
    Error {
        /// Typed safe failure class.
        failure: DiscoveryFailure,
    },
    /// One successful model and mode observation.
    Ready {
        /// Shared across subscribers and host-scoped workspace targets.
        catalogue: Arc<Catalogue>,
        /// Wall-clock observation time, separate from content identity.
        fetched_at: DateTime<Utc>,
    },
}

/// Reader-safe record, structurally unable to serialize launch secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProviderSnapshotEntry {
    /// Effective provider identity.
    pub provider: ProviderId,
    /// Implementation family, independent from configured identity.
    pub kind: ProviderKind,
    /// Human label; shareable reports must normalize this further.
    pub label: AgentLabel,
    /// Explicit participation state.
    pub enabled: Enabled,
    /// Built-in or custom source.
    pub source: SnapshotSource,
    /// Observed availability and catalogue.
    pub state: ProviderState,
}

/// Read authorization and side-effect intent are explicit at every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DiagnosticRead {
    /// Read permission, no process/filesystem/network discovery.
    Cached,
    /// Operate permission, explicit bounded discovery permitted.
    Refresh,
}

/// Unregistered provider diagnostic request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetProviderDiagnostic {
    /// Effective identity.
    pub provider: ProviderId,
    /// Target for providers whose catalogue depends on a workspace.
    pub scope: CatalogueScope,
    /// A cache miss never silently upgrades this intent.
    pub read: DiagnosticRead,
}

/// Same cache projection as snapshots; shareable reports normalize names/version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProviderDiagnostic {
    /// Reader-safe metadata and observation.
    pub entry: ProviderSnapshotEntry,
    /// Already-observed bounded version, no implicit version command.
    pub version: Option<AgentVersion>,
}

/// Unregistered snapshot request; active discovery needs Operate authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetProviders {
    /// Explicit target scope.
    pub scope: CatalogueScope,
    /// Cache-only reads start no provider.
    pub read: DiagnosticRead,
}

/// Unregistered pure preflight; the node must derive parent authority from the
/// authenticated caller instead of treating client-supplied values as a grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResolveProviderCreate {
    /// Policy parameters, not execution authority or session mutation.
    pub input: CreateInput,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passive_request_roundtrips_without_registering_any_method() {
        assert!(serde_json::from_str::<CatalogueDirectory>(r#""relative""#).is_err());
        let request = GetProviderDiagnostic {
            provider: "claude".parse().unwrap(),
            scope: CatalogueScope::Global,
            read: DiagnosticRead::Cached,
        };
        assert_eq!(
            serde_json::from_str::<GetProviderDiagnostic>(
                &serde_json::to_string(&request).unwrap()
            )
            .unwrap(),
            request
        );
        assert_eq!(super::super::PROVIDERS, None);
    }

    #[cfg(feature = "ts")]
    #[test]
    fn generated_catalogue_shape_never_exports_its_private_thinking_index() {
        let mut declarations = crate::ts::Declarations::new();
        declarations.add::<ProviderDiagnostic>().unwrap();
        declarations.add::<GetProviders>().unwrap();
        let generated = declarations.render(None);
        assert!(generated.contains("ProviderThinkingChoice"));
        assert!(!generated.contains("ThinkingSet"));
        assert!(!generated.contains("ids:"));
        assert!(generated.contains("ProviderCatalogue"));
        assert!(generated.contains("CatalogueScope"));
    }
}
