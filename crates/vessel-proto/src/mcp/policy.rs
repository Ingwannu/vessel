//! Launch snapshots reproduce a provider denylist without broadening authority.

use serde::{Deserialize, Deserializer, Serialize};

use super::{InvalidMcp, StoredTool, ToolId, bounded};
use crate::HarnessRef;

/// Maximum disabled identifiers per provider; a stored list is bounded too.
pub const MAX_DISABLED_TOOLS: usize = 256;
/// Maximum exact-provider overrides in one node's policy.
pub const MAX_PROVIDER_POLICIES: usize = 64;

/// Whether a launch gets the internal tool server; false is fresh-install behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "bool", into = "bool")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "boolean"))]
pub enum Injection {
    /// No internal server is attached.
    #[default]
    Off,
    /// Attach the internal server using the child's provider policy.
    On,
}
impl From<bool> for Injection {
    fn from(on: bool) -> Self {
        if on { Self::On } else { Self::Off }
    }
}
impl From<Injection> for bool {
    fn from(value: Injection) -> Self {
        matches!(value, Injection::On)
    }
}

/// Provider policy defaults on, independently from the global inject switch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "bool", into = "bool")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "boolean"))]
pub enum ToolAccess {
    /// Offer tools except exact denied names.
    #[default]
    Enabled,
    /// Offer no tools for this provider.
    Disabled,
}
impl From<bool> for ToolAccess {
    fn from(on: bool) -> Self {
        if on { Self::Enabled } else { Self::Disabled }
    }
}
impl From<ToolAccess> for bool {
    fn from(value: ToolAccess) -> Self {
        matches!(value, ToolAccess::Enabled)
    }
}

/// Denylist frozen at launch; unknown stored names survive without disabling a tool here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PolicyInput")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ToolPolicy {
    enabled: ToolAccess,
    disabled: Vec<StoredTool>,
}

#[derive(Deserialize)]
struct PolicyInput {
    #[serde(default)]
    enabled: ToolAccess,
    #[serde(
        default,
        deserialize_with = "bounded::decode::<_, _, MAX_DISABLED_TOOLS>"
    )]
    disabled: Vec<StoredTool>,
}
impl TryFrom<PolicyInput> for ToolPolicy {
    type Error = InvalidMcp;
    fn try_from(input: PolicyInput) -> Result<Self, Self::Error> {
        Self::new(input.enabled, input.disabled)
    }
}

impl ToolPolicy {
    /// Refuse excessive or duplicate entries before storing a launch choice.
    ///
    /// # Errors
    /// Excessive or repeated denylist entries are refused.
    pub fn new(enabled: ToolAccess, disabled: Vec<StoredTool>) -> Result<Self, InvalidMcp> {
        if disabled.len() > MAX_DISABLED_TOOLS {
            return Err(InvalidMcp::DisabledCount);
        }
        let mut names = std::collections::HashSet::new();
        if disabled.iter().any(|tool| !names.insert(tool.as_str())) {
            return Err(InvalidMcp::DuplicateTool);
        }
        Ok(Self { enabled, disabled })
    }

    /// Injection omits the whole server when this provider explicitly disables it.
    #[must_use]
    pub const fn access(&self) -> ToolAccess {
        self.enabled
    }

    /// Visibility and invocation must ask the same predicate.
    #[must_use]
    pub fn permits(&self, tool: ToolId) -> bool {
        self.enabled == ToolAccess::Enabled
            && !self
                .disabled
                .iter()
                .any(|entry| matches!(entry, StoredTool::Known(id) if *id == tool))
    }

    /// Reads preserve future names; new edits must not claim to enforce one.
    ///
    /// # Errors
    /// A future tool name cannot be newly disabled by this build.
    pub fn validate_edit(&self) -> Result<(), InvalidMcp> {
        if self
            .disabled
            .iter()
            .any(|entry| matches!(entry, StoredTool::Unknown(_)))
        {
            return Err(InvalidMcp::UnknownTool);
        }
        Ok(())
    }
}

/// Policy for exactly one provider, never inherited from another provider's base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProviderPolicy {
    /// Known or configured provider identity.
    pub harness: HarnessRef,
    /// Denylist applied to this provider's launches.
    #[serde(flatten)]
    pub policy: ToolPolicy,
}

/// Bounded unique providers; prevents ambiguous policy selection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProviderPolicies(Vec<ProviderPolicy>);
impl ProviderPolicies {
    /// Validate count and uniqueness before constructing a node policy.
    ///
    /// # Errors
    /// Excessive or repeated provider overrides are refused.
    pub fn new(items: Vec<ProviderPolicy>) -> Result<Self, InvalidMcp> {
        if items.len() > MAX_PROVIDER_POLICIES {
            return Err(InvalidMcp::ProviderCount);
        }
        let mut found = std::collections::HashSet::new();
        if items.iter().any(|item| !found.insert(&item.harness)) {
            return Err(InvalidMcp::DuplicateProvider);
        }
        Ok(Self(items))
    }

    /// An absent exact-provider override means every known tool is enabled.
    #[must_use]
    pub fn for_harness(&self, harness: &HarnessRef) -> ToolPolicy {
        self.0
            .iter()
            .find(|entry| entry.harness == *harness)
            .map(|entry| entry.policy.clone())
            .unwrap_or_default()
    }

    /// Reject unknown new edits without losing future names in saved settings.
    ///
    /// # Errors
    /// A future tool name cannot be newly disabled by this build.
    pub fn validate_edit(&self) -> Result<(), InvalidMcp> {
        self.0
            .iter()
            .try_for_each(|entry| entry.policy.validate_edit())
    }
}
impl<'de> Deserialize<'de> for ProviderPolicies {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let items = bounded::decode::<_, _, MAX_PROVIDER_POLICIES>(input)?;
        Self::new(items).map_err(D::Error::custom)
    }
}

/// Global injection plus exact-provider policies, frozen on the next launch.
/// No HTTP listener exists in this package; injection uses stdio.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct McpSettings {
    /// Fresh nodes opt in; upgrade migration writes the earlier on behavior.
    #[serde(default)]
    pub inject: Injection,
    /// Providers without an override offer every tool.
    #[serde(default)]
    pub policy: ProviderPolicies,
}

/// A partial edit never clears another sub-field when toggling injection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct McpSettingsPatch {
    /// Omission keeps the prior decision, including an upgraded node's On.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inject: Option<Injection>,
    /// Omission keeps the exact provider overrides and future stored names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<ProviderPolicies>,
}
impl McpSettingsPatch {
    /// Resolve a stored patch over fresh defaults without guessing upgrade state.
    #[must_use]
    pub fn settings(&self) -> McpSettings {
        McpSettings {
            inject: self.inject.unwrap_or_default(),
            policy: self.policy.clone().unwrap_or_default(),
        }
    }
    /// Preserve unmentioned fields, including an explicit false migration marker.
    #[must_use]
    pub fn over(self, previous: Self) -> Self {
        Self {
            inject: self.inject.or(previous.inject),
            policy: self.policy.or(previous.policy),
        }
    }
    /// New denylist edits must be enforceable by this node.
    ///
    /// # Errors
    /// Unknown tools in a newly supplied policy are refused before persistence.
    pub fn validate_edit(&self) -> Result<(), InvalidMcp> {
        if let Some(policy) = &self.policy {
            policy.validate_edit()?;
        }
        Ok(())
    }
}
