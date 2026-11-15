//! Reference mode tables. They are policy metadata, never an availability probe.

use vessel_proto::{
    agents::AgentLabel,
    providers::{
        ApprovalFlow, CreatePolicy, Enabled, InvalidProvider, Mode, ModeCatalogue, ModeRisk,
        ProviderId, ProviderKind,
    },
};

/// One built-in definition with its implementation family and permission policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Builtin {
    /// Effective manifest identity, before configured aliases are applied.
    pub id: ProviderId,
    /// Implementation family; it does not mean a driver has been built.
    pub kind: ProviderKind,
    /// Human display name.
    pub label: AgentLabel,
    /// Opt-in state before configuration overrides.
    pub enabled: Enabled,
    /// Reference modes; a dynamic catalogue may replace these.
    pub modes: ModeCatalogue,
    /// Provider-specific inheritance semantics.
    pub create_policy: CreatePolicy,
}

/// Match the canonical built-in namespace without inventing another name table.
/// Legacy decoding remains separate so saved added agents can still be removed.
///
/// # Errors
/// The same compiled manifest error as [`builtin_manifest`]; callers must fail
/// closed rather than admitting a name while the manifest is unavailable.
pub fn builtin_named(name: &str) -> Result<Option<ProviderId>, ManifestError> {
    Ok(builtin_manifest()?
        .into_iter()
        .find(|builtin| builtin.id.as_str().eq_ignore_ascii_case(name))
        .map(|builtin| builtin.id))
}

/// Reference built-ins in provider-manifest order. Never includes a test provider.
///
/// # Errors
/// Compiled text or duplicate mode metadata violates the protocol value contract.
pub fn builtin_manifest() -> Result<Vec<Builtin>, ManifestError> {
    use ApprovalFlow::{Interactive as Ask, Unattended as Free};
    use ModeRisk::{Dangerous, Moderate, Planning, Safe};
    Ok(vec![
        entry(
            "claude",
            "Claude",
            ProviderKind::Claude,
            Enabled::On,
            Some("auto"),
            &[
                ("plan", "Plan Mode", Planning, Ask),
                ("default", "Always Ask", Safe, Ask),
                ("acceptEdits", "Accept File Edits", Moderate, Ask),
                ("auto", "Auto mode", Moderate, Ask),
                ("bypassPermissions", "Bypass", Dangerous, Free),
            ],
        )?,
        entry(
            "codex",
            "Codex",
            ProviderKind::Codex,
            Enabled::On,
            Some("auto-review"),
            &[
                ("auto", "Default Permissions", Moderate, Ask),
                ("auto-review", "Auto-review", Moderate, Ask),
                ("full-access", "Full Access", Dangerous, Free),
            ],
        )?,
        entry(
            "copilot",
            "Copilot",
            ProviderKind::Copilot,
            Enabled::On,
            Some(COPILOT_AGENT),
            &[
                (COPILOT_AGENT, "Agent", Moderate, Ask),
                (COPILOT_PLAN, "Plan", Planning, Ask),
                ("allow-all", "Allow All", Dangerous, Free),
            ],
        )?,
        entry(
            "opencode",
            "OpenCode",
            ProviderKind::OpenCode,
            Enabled::On,
            None,
            &[
                ("build", "Build", Moderate, Ask),
                ("plan", "Plan", Planning, Ask),
            ],
        )?,
        entry("pi", "Pi", ProviderKind::Pi, Enabled::On, None, &[])?,
        entry(
            "omp",
            "Oh My Pi",
            ProviderKind::Omp,
            Enabled::Off,
            Some("full"),
            &[
                ("full", "Full Access", Dangerous, Free),
                ("write", "Write Approval", Moderate, Ask),
                ("ask", "Always Ask", Safe, Ask),
            ],
        )?,
    ])
}

const COPILOT_AGENT: &str = "https://agentclientprotocol.com/protocol/session-modes#agent";
const COPILOT_PLAN: &str = "https://agentclientprotocol.com/protocol/session-modes#plan";

/// Static manifest construction failed; keeps the original typed text error.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// An identity, default or mode list is invalid.
    #[error(transparent)]
    Value(#[from] InvalidProvider),
    /// A reference label is invalid.
    #[error(transparent)]
    Label(#[from] vessel_proto::agents::InvalidAgentText),
}

fn entry(
    id: &str,
    label: &str,
    kind: ProviderKind,
    enabled: Enabled,
    default: Option<&str>,
    rows: &[(&str, &str, ModeRisk, ApprovalFlow)],
) -> Result<Builtin, ManifestError> {
    let modes = rows
        .iter()
        .map(|(id, label, risk, approval)| {
            Ok(Mode {
                id: id.parse()?,
                label: label.parse()?,
                risk: *risk,
                approval: *approval,
            })
        })
        .collect::<Result<Vec<_>, ManifestError>>()?;
    let create_policy = match kind {
        ProviderKind::OpenCode => CreatePolicy::OpenCode,
        ProviderKind::Copilot | ProviderKind::Acp => CreatePolicy::Acp,
        ProviderKind::Claude
        | ProviderKind::Codex
        | ProviderKind::Pi
        | ProviderKind::Omp
        | ProviderKind::Plugin => CreatePolicy::Default,
    };
    Ok(Builtin {
        id: id.parse()?,
        label: label.parse()?,
        kind,
        enabled,
        modes: ModeCatalogue::new(modes, default.map(str::parse).transpose()?)?,
        create_policy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_order_defaults_and_unattended_flags_are_retained() {
        let rows = builtin_manifest().unwrap();
        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["claude", "codex", "copilot", "opencode", "pi", "omp"]
        );
        assert_eq!(
            rows.iter()
                .map(|r| r
                    .modes
                    .default_mode()
                    .map(vessel_proto::providers::ModeId::as_str))
                .collect::<Vec<_>>(),
            [
                Some("auto"),
                Some("auto-review"),
                Some(COPILOT_AGENT),
                None,
                None,
                Some("full")
            ]
        );
        assert_eq!(rows.last().unwrap().enabled, Enabled::Off);
        assert!(
            rows.iter()
                .take(rows.len() - 1)
                .all(|r| r.enabled == Enabled::On)
        );
        assert_eq!(
            rows.iter()
                .map(|r| r
                    .modes
                    .modes()
                    .iter()
                    .filter(|m| m.approval == ApprovalFlow::Unattended)
                    .count())
                .collect::<Vec<_>>(),
            [1, 1, 1, 0, 0, 1]
        );
        assert!(rows.iter().all(|r| !r.id.as_str().starts_with("mock")));
    }
    #[test]
    fn built_in_names_share_one_case_insensitive_namespace_without_prefix_matching() {
        for builtin in builtin_manifest().unwrap() {
            for spelling in [
                builtin.id.as_str().to_owned(),
                builtin.id.as_str().to_uppercase(),
            ] {
                assert_eq!(builtin_named(&spelling).unwrap(), Some(builtin.id.clone()));
            }
        }
        for name in ["pi-custom", "Copilot2", "goose", "omp-profile"] {
            assert!(builtin_named(name).unwrap().is_none());
        }
    }
}
