//! Pure provider registry generation. Plugin definitions are an inert open input;
//! this module neither trusts nor starts a plugin or provider runtime.

use super::{Builtin, ManifestError, ProviderLaunchError, builtin_manifest, merge_launch};
use std::collections::{BTreeMap, BTreeSet};
use vessel_proto::{
    agents::AgentLabel,
    providers::{
        ConfigurationError, CreatePolicy, Enabled, LaunchSettings, MAX_PROVIDERS, ModeCatalogue,
        Models, ProviderId, ProviderKind, ProviderOverride, ProviderOverrides,
    },
};

/// Whether a plugin is usable without an explicit provider override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationRequirement {
    /// Registration alone enables the provider.
    Optional,
    /// A configuration entry must exist before a client can be built.
    Required,
}

/// Identity of a plugin registration generation, assigned by its runtime owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginRevision(u64);

impl PluginRevision {
    /// Runtime-owned revision; replacement must use a different value.
    #[must_use]
    pub const fn new(revision: u64) -> Self {
        Self(revision)
    }
}

/// Contribution hook for the trusted plugin runtime. No callback or executable
/// crosses this value boundary; connection ownership stays with that runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginDefinition {
    /// Effective provider identity, checked for collisions with built-ins.
    pub id: ProviderId,
    /// Human display name.
    pub label: AgentLabel,
    /// Missing configuration can intentionally suppress this provider.
    pub configuration: ConfigurationRequirement,
    /// Used by the catalogue owner to discard superseded results.
    pub revision: PluginRevision,
    /// Launch settings resolved by the same policy as ordinary providers.
    pub launch: LaunchSettings,
}

/// Origin is separate from family: aliases of a built-in are custom providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderSource {
    /// Reference manifest entry.
    Builtin,
    /// A configured alias of a built-in or configured ACP provider.
    Configured,
    /// Trusted runtime registration, with replacement identity.
    Plugin(PluginRevision),
}

/// Effective definition, safe to compare when deciding which caches to retire.
/// It contains configuration and must never be returned as a reader snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDefinition {
    /// Effective identity, used by every catalogue and child configuration.
    pub id: ProviderId,
    /// Native implementation family.
    pub kind: ProviderKind,
    /// Person-facing label.
    pub label: AgentLabel,
    /// Discovery/creation opt-in.
    pub enabled: Enabled,
    /// Manifest modes, not fabricated runtime discovery.
    pub modes: ModeCatalogue,
    /// Default, ACP or `OpenCode` inheritance semantics.
    pub create_policy: CreatePolicy,
    /// Inherited command, environment and denied-tool policy.
    pub launch: LaunchSettings,
    /// Only nonempty replacement lists suppress model discovery.
    pub models: Option<Models>,
    /// Appended/overridden catalogue definitions.
    pub additional_models: Models,
    /// Built-in identity is retained for configured aliases.
    pub derived_from: Option<ProviderId>,
    /// Controls invalidation on plugin registration replacement.
    pub source: ProviderSource,
}

impl ProviderDefinition {
    /// Host-scoped Claude/Codex discovery is shared across workspaces, as in
    /// Paseo's client cache-key contract. Other providers retain the target.
    #[must_use]
    pub fn catalogue_key(
        &self,
        target: &vessel_proto::providers::CatalogueScope,
    ) -> vessel_proto::providers::CatalogueScope {
        use vessel_proto::providers::CatalogueScope;
        match self.kind {
            ProviderKind::Claude | ProviderKind::Codex => CatalogueScope::Global,
            ProviderKind::Copilot
            | ProviderKind::OpenCode
            | ProviderKind::Pi
            | ProviderKind::Omp
            | ProviderKind::Acp
            | ProviderKind::Plugin => target.clone(),
        }
    }

    /// Project inert configuration into reader-safe metadata without exposing
    /// the command, environment, denied tools or plugin revision.
    #[must_use]
    pub fn snapshot_entry(
        &self,
        state: vessel_proto::providers::ProviderState,
    ) -> vessel_proto::providers::ProviderSnapshotEntry {
        use vessel_proto::providers::{ProviderSnapshotEntry, SnapshotSource};
        let source = match self.source {
            ProviderSource::Builtin => SnapshotSource::Builtin,
            ProviderSource::Configured | ProviderSource::Plugin(_) => SnapshotSource::Custom,
        };
        ProviderSnapshotEntry {
            provider: self.id.clone(),
            kind: self.kind,
            label: self.label.clone(),
            enabled: self.enabled,
            source,
            state,
        }
    }
}

/// Ambiguous or invalid generations are refused as a whole before any client retires.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// Invalid compiled reference metadata.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// Launch inheritance could not preserve bounds and invariants.
    #[error(transparent)]
    Launch(#[from] ProviderLaunchError),
    /// Too many definitions for the node's bounded registry.
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    /// Plugin identity cannot replace a built-in implementation.
    #[error("plugin provider '{0}' conflicts with a built-in provider")]
    BuiltinCollision(ProviderId),
    /// One plugin generation cannot register an identity twice.
    #[error("duplicate plugin provider '{0}'")]
    DuplicatePlugin(ProviderId),
    /// Derived providers have an explicit name in Paseo's configuration grammar.
    #[error("custom provider '{0}' must declare a label")]
    MissingLabel(ProviderId),
    /// Generic ACP is meaningless without an executable supplied by the owner.
    #[error("ACP provider '{0}' requires a replacement command")]
    MissingCommand(ProviderId),
    /// Only built-ins or ACP are permitted inheritance bases.
    #[error("custom provider '{0}' cannot extend a plugin")]
    InvalidBase(ProviderId),
    /// Building an empty mode catalogue violated its contract.
    #[error(transparent)]
    Modes(#[from] vessel_proto::providers::InvalidProvider),
}

/// Ordered immutable generation. Unknown plugin-only overrides are retained as
/// unresolved IDs so the caller can surface a warning after plugin settlement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRegistry {
    definitions: Vec<ProviderDefinition>,
    unresolved: Vec<ProviderId>,
    shadowed_plugins: Vec<ProviderId>,
}

impl ProviderRegistry {
    /// Build and validate a complete generation before replacing the previous one.
    ///
    /// # Errors
    /// `RegistryError` rejects collisions, missing required data or excessive input.
    pub fn build(
        overrides: &ProviderOverrides,
        plugins: &[PluginDefinition],
    ) -> Result<Self, RegistryError> {
        if plugins.len() > MAX_PROVIDERS {
            return Err(ConfigurationError::TooMany.into());
        }
        let mut definitions = builtin_manifest()?
            .into_iter()
            .map(from_builtin)
            .collect::<Vec<_>>();
        for definition in &mut definitions {
            if let Some(overlay) = overrides.entries().get(&definition.id) {
                apply(definition, overlay)?;
            }
        }
        let mut registry = Self {
            definitions,
            unresolved: Vec::new(),
            shadowed_plugins: Vec::new(),
        };
        registry.add_plugins(overrides, plugins)?;
        registry.add_derived(overrides)?;
        if registry.definitions.len() > MAX_PROVIDERS {
            return Err(ConfigurationError::TooMany.into());
        }
        Ok(registry)
    }

    /// Reference order followed by plugins and configured providers.
    #[must_use]
    pub fn definitions(&self) -> &[ProviderDefinition] {
        &self.definitions
    }

    /// Lookup by effective identity, never by implementation family.
    #[must_use]
    pub fn get(&self, id: &ProviderId) -> Option<&ProviderDefinition> {
        self.definitions.iter().find(|d| &d.id == id)
    }

    /// Orphan plugin overrides are reported after plugin registration settles.
    #[must_use]
    pub fn unresolved(&self) -> &[ProviderId] {
        &self.unresolved
    }

    /// A configured extends declaration intentionally wins over a plugin.
    #[must_use]
    pub fn shadowed_plugins(&self) -> &[ProviderId] {
        &self.shadowed_plugins
    }

    fn add_plugins(
        &mut self,
        overrides: &ProviderOverrides,
        plugins: &[PluginDefinition],
    ) -> Result<(), RegistryError> {
        let mut seen = BTreeSet::new();
        for plugin in plugins {
            if !seen.insert(&plugin.id) {
                return Err(RegistryError::DuplicatePlugin(plugin.id.clone()));
            }
            if self.get(&plugin.id).is_some() {
                return Err(RegistryError::BuiltinCollision(plugin.id.clone()));
            }
            let overlay = overrides.entries().get(&plugin.id);
            if overlay.is_none() && plugin.configuration == ConfigurationRequirement::Required {
                continue;
            }
            if overlay.is_some_and(|o| o.extends.is_some()) {
                self.shadowed_plugins.push(plugin.id.clone());
                continue;
            }
            let mut definition = ProviderDefinition {
                id: plugin.id.clone(),
                kind: ProviderKind::Plugin,
                label: plugin.label.clone(),
                enabled: Enabled::On,
                modes: ModeCatalogue::new(vec![], None)?,
                create_policy: CreatePolicy::Default,
                launch: plugin.launch.clone(),
                models: None,
                additional_models: Models::default(),
                derived_from: None,
                source: ProviderSource::Plugin(plugin.revision),
            };
            if let Some(overlay) = overlay {
                apply(&mut definition, overlay)?;
            }
            self.definitions.push(definition);
        }
        Ok(())
    }

    fn add_derived(&mut self, overrides: &ProviderOverrides) -> Result<(), RegistryError> {
        // Clone only the finite base-definition index: a custom provider cannot
        // inherit another custom provider through configuration ordering.
        let bases: BTreeMap<_, _> = self
            .definitions
            .iter()
            .filter(|d| d.source == ProviderSource::Builtin)
            .map(|d| (d.kind, d.clone()))
            .collect();
        for (id, overlay) in overrides.entries() {
            if self.get(id).is_some() {
                continue;
            }
            let Some(base) = overlay.extends else {
                self.unresolved.push(id.clone());
                continue;
            };
            let label = overlay
                .label
                .as_ref()
                .ok_or_else(|| RegistryError::MissingLabel(id.clone()))?;
            let mut definition = match base {
                ProviderKind::Acp => {
                    if !matches!(
                        overlay.command,
                        Some(vessel_proto::providers::CommandEdit::Replace { .. })
                    ) {
                        return Err(RegistryError::MissingCommand(id.clone()));
                    }
                    ProviderDefinition {
                        id: id.clone(),
                        kind: base,
                        label: label.clone(),
                        enabled: Enabled::On,
                        modes: ModeCatalogue::new(vec![], None)?,
                        create_policy: CreatePolicy::Acp,
                        launch: LaunchSettings::default(),
                        models: None,
                        additional_models: Models::default(),
                        derived_from: None,
                        source: ProviderSource::Configured,
                    }
                }
                ProviderKind::Plugin => return Err(RegistryError::InvalidBase(id.clone())),
                ProviderKind::Claude
                | ProviderKind::Codex
                | ProviderKind::Copilot
                | ProviderKind::OpenCode
                | ProviderKind::Pi
                | ProviderKind::Omp => {
                    let Some(base) = bases.get(&base) else {
                        return Err(RegistryError::InvalidBase(id.clone()));
                    };
                    let mut derived = base.clone();
                    derived.id = id.clone();
                    derived.label = label.clone();
                    derived.enabled = Enabled::On;
                    derived.derived_from = Some(base.id.clone());
                    derived.models = None;
                    derived.additional_models = Models::default();
                    derived.source = ProviderSource::Configured;
                    derived
                }
            };
            apply(&mut definition, overlay)?;
            self.definitions.push(definition);
        }
        Ok(())
    }
}

fn from_builtin(builtin: Builtin) -> ProviderDefinition {
    ProviderDefinition {
        id: builtin.id,
        kind: builtin.kind,
        label: builtin.label,
        enabled: builtin.enabled,
        modes: builtin.modes,
        create_policy: builtin.create_policy,
        launch: LaunchSettings::default(),
        models: None,
        additional_models: Models::default(),
        derived_from: None,
        source: ProviderSource::Builtin,
    }
}

fn apply(
    definition: &mut ProviderDefinition,
    overlay: &ProviderOverride,
) -> Result<(), RegistryError> {
    definition.launch = merge_launch(
        &definition.launch,
        overlay.command.as_ref(),
        &overlay.env,
        &overlay.disallowed_tools,
    )?;
    if let Some(label) = &overlay.label {
        definition.label = label.clone();
    }
    if let Some(enabled) = overlay.enabled {
        definition.enabled = enabled;
    }
    definition.models = overlay
        .models
        .as_ref()
        .filter(|models| !models.all().is_empty())
        .cloned();
    definition.additional_models = overlay.additional_models.clone();
    Ok(())
}

#[cfg(test)]
mod tests;
