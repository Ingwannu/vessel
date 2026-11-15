//! Provider create inheritance, with explicit choices ahead of unattended policy.
//! This resolver produces values only: it cannot grant execution authority.

use vessel_proto::providers::{
    ApprovalFlow, AutoAccept, CreateConfig, CreateInput, CreatePolicy, ModeCatalogue, ModeId,
    ProviderId,
};

/// Refusal before launch, retaining structured choices instead of matching prose.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CreateError {
    /// Explicit mode is not among the provider's reported modes.
    #[error("requested mode '{mode}' is unavailable for provider '{provider}'")]
    InvalidMode {
        /// Effective target identity.
        provider: ProviderId,
        /// The explicit choice.
        mode: ModeId,
        /// Known modes, or no catalogue when discovery has not completed.
        available: Option<ModeCatalogue>,
    },
    /// Different providers' interactive permission modes have no implicit mapping.
    #[error(
        "cannot inherit a mode from provider '{parent}' for provider '{target}'; pass an explicit mode"
    )]
    CannotInherit {
        /// Parent's effective identity.
        parent: ProviderId,
        /// Target's effective identity.
        target: ProviderId,
        /// Parent's native mode, when it had one.
        mode: Option<ModeId>,
        /// Catalogue for a caller to present actionable choices.
        available: Option<ModeCatalogue>,
    },
    /// A compiled compatibility identifier could not be represented.
    #[error("provider compatibility mode is invalid")]
    InvalidCompatibility(#[from] vessel_proto::providers::InvalidProvider),
}

/// Resolve provider policy before launching a process. The snapshot owner must
/// supply the actual ready catalogue, never a manifest as fabricated discovery.
///
/// # Errors
/// Explicit invalid choices and ambiguous cross-provider inheritance are refused.
pub fn resolve_create(
    policy: CreatePolicy,
    input: &CreateInput,
) -> Result<CreateConfig, CreateError> {
    let unattended = input.approval == ApprovalFlow::Unattended
        || input
            .parent
            .as_ref()
            .is_some_and(|p| p.approval == ApprovalFlow::Unattended);
    let mut mode = input.mode.clone();
    let mut auto_accept = input.auto_accept;
    match policy {
        CreatePolicy::Default => {}
        CreatePolicy::OpenCode => {
            let legacy: ModeId = "full-access".parse()?;
            if mode.as_ref() == Some(&legacy) {
                mode = Some("build".parse()?);
                auto_accept = Some(AutoAccept::On);
            } else if unattended && auto_accept.is_none() {
                auto_accept = Some(AutoAccept::On);
            }
            if input.mode.is_none() && unattended {
                mode = input
                    .parent
                    .as_ref()
                    .filter(|p| p.provider == input.provider)
                    .and_then(|p| p.mode.clone());
                if mode.is_none() {
                    return Ok(finish(policy, None, auto_accept, input));
                }
            }
        }
        CreatePolicy::Acp => {
            if unattended && auto_accept.is_none() {
                auto_accept = Some(AutoAccept::On);
            }
            if input.mode.is_none()
                && unattended
                && input
                    .parent
                    .as_ref()
                    .is_some_and(|p| p.provider != input.provider)
            {
                return Ok(finish(policy, None, auto_accept, input));
            }
        }
    }
    let mode = resolve_mode(input, mode.as_ref())?;
    Ok(finish(policy, mode, auto_accept, input))
}

fn finish(
    policy: CreatePolicy,
    mode: Option<ModeId>,
    auto_accept: Option<AutoAccept>,
    input: &CreateInput,
) -> CreateConfig {
    let mode_unattended = input.available.as_ref().is_some_and(|catalogue| {
        catalogue.modes().iter().any(|candidate| {
            Some(&candidate.id) == mode.as_ref() && candidate.approval == ApprovalFlow::Unattended
        })
    });
    let feature_unattended = match policy {
        CreatePolicy::Default => false,
        CreatePolicy::OpenCode | CreatePolicy::Acp => auto_accept == Some(AutoAccept::On),
    };
    let approval = if mode_unattended || feature_unattended {
        ApprovalFlow::Unattended
    } else {
        ApprovalFlow::Interactive
    };
    CreateConfig {
        mode,
        auto_accept,
        approval,
    }
}

fn resolve_mode(
    input: &CreateInput,
    explicit: Option<&ModeId>,
) -> Result<Option<ModeId>, CreateError> {
    if let Some(mode) = explicit {
        if input
            .available
            .as_ref()
            .is_some_and(|c| !c.modes().iter().any(|m| &m.id == mode))
        {
            return Err(CreateError::InvalidMode {
                provider: input.provider.clone(),
                mode: mode.clone(),
                available: input.available.clone(),
            });
        }
        return Ok(Some(mode.clone()));
    }
    if let Some(parent) = &input.parent
        && parent.provider == input.provider
    {
        return Ok(parent.mode.clone());
    }
    let unattended = input.approval == ApprovalFlow::Unattended
        || input
            .parent
            .as_ref()
            .is_some_and(|p| p.approval == ApprovalFlow::Unattended);
    if unattended
        && let Some(mode) = input.available.as_ref().and_then(|c| {
            c.modes()
                .iter()
                .find(|m| m.approval == ApprovalFlow::Unattended)
        })
    {
        return Ok(Some(mode.id.clone()));
    }
    if input.parent.is_none()
        || input
            .available
            .as_ref()
            .is_some_and(|c| c.modes().is_empty())
    {
        return Ok(None);
    }
    let Some(parent) = &input.parent else {
        return Ok(None);
    };
    Err(CreateError::CannotInherit {
        parent: parent.provider.clone(),
        target: input.provider.clone(),
        mode: parent.mode.clone(),
        available: input.available.clone(),
    })
}

#[cfg(test)]
mod tests;
