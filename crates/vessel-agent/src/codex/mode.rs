//! The session's mode over Codex's own protocol: what Codex asks a person
//! before acting, and in which sandbox it acts (the plan's Codex slice 2).
//!
//! [Decision Log]
//! - Purpose: the mode decides which approvals reach a person, so the
//!   permissions slice carries it; the other options are slice 3's.
//! - Choice: native options use Paseo's `auto`, `auto-review`, `full-access`.
//!   `read-only` is accepted but not offered. Old native spellings remain decode
//!   aliases with their original policy; actual ACP observations stay opaque.
//!   Auto-review is offered/default only at the observed 0.115.0 floor. Unknown
//!   versions use Default Permissions and cannot select auto-review.
//! - Full access (`never` asking, `danger-full-access`) is set only by
//!   its own value: no other value maps to either.
//! - A change applies from the next turn (`turn/start` carries the
//!   policy, the reviewer and the sandbox), as codex-acp applies it.

use vessel_proto::{
    ChoiceValue, OptionCategory, OptionChoice, OptionId, OptionKind, OptionValue, OptionValueId,
    SessionOption,
};

use super::policy::CodexVersionPolicy;
use super::protocol::{ApprovalsReviewer, AskForApproval, SandboxMode, SandboxPolicy};
use crate::options::{KindName, OptionError};

/// The option's id, as codex-acp names it.
pub(super) const OPTION: &str = "mode";

/// A mode Codex runs a session in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    ReadOnly,
    WorkspaceWrite,
    Agent,
    AgentFullAccess,
}

impl Mode {
    /// Every accepted native policy, including the unoffered read-only preset.
    #[cfg(test)]
    pub(super) const ALL: [Self; 4] = [
        Self::ReadOnly,
        Self::WorkspaceWrite,
        Self::Agent,
        Self::AgentFullAccess,
    ];

    /// Conservative fallback before version observation.
    pub(super) const DEFAULT: Self = Self::WorkspaceWrite;

    pub(super) fn default_for(policy: CodexVersionPolicy) -> Self {
        if policy.auto_review() {
            Self::Agent
        } else {
            Self::WorkspaceWrite
        }
    }

    fn offered(policy: CodexVersionPolicy) -> impl Iterator<Item = Self> {
        [Self::WorkspaceWrite, Self::Agent, Self::AgentFullAccess]
            .into_iter()
            .filter(move |mode| *mode != Self::Agent || policy.auto_review())
    }

    /// Its value's id.
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::WorkspaceWrite => "auto",
            Self::Agent => "auto-review",
            Self::AgentFullAccess => "full-access",
        }
    }

    /// Native value or an explicit legacy native alias, never an ACP remap.
    pub(super) fn named(id: &str) -> Option<Self> {
        match id {
            "read-only" => Some(Self::ReadOnly),
            "auto" | "workspace-write" => Some(Self::WorkspaceWrite),
            "auto-review" | "agent" => Some(Self::Agent),
            "full-access" | "agent-full-access" => Some(Self::AgentFullAccess),
            _ => None,
        }
    }

    /// The mode `choice` picks: an error, as an ACP agent's, for another
    /// option, another value or a toggle.
    pub(super) fn chosen(
        choice: &OptionChoice,
        policy: CodexVersionPolicy,
    ) -> Result<Self, OptionError> {
        let id = &choice.id;
        if id.as_str() != OPTION {
            return Err(OptionError::Unknown {
                id: id.clone(),
                offered: vec![OptionId::new(OPTION)],
            });
        }
        match &choice.value {
            ChoiceValue::Select { value } => Self::named(value.as_str())
                .filter(|mode| *mode != Self::Agent || policy.auto_review())
                .ok_or_else(|| OptionError::NotOffered {
                    id: id.clone(),
                    value: value.clone(),
                    offered: Self::offered(policy)
                        .map(|mode| OptionValueId::new(mode.id()))
                        .collect(),
                }),
            ChoiceValue::Toggle { .. } => Err(OptionError::WrongKind {
                id: id.clone(),
                kind: KindName::Select,
            }),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::ReadOnly => "Read-only",
            Self::WorkspaceWrite => "Default Permissions",
            Self::Agent => "Auto-review",
            Self::AgentFullAccess => "Full Access",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::ReadOnly => "Requires approval to edit files and access the internet.",
            Self::WorkspaceWrite => {
                "Edit workspace files; ask before writing outside the workspace or accessing the \
                 network."
            }
            Self::Agent => "Only ask for actions detected as potentially unsafe",
            Self::AgentFullAccess => {
                "Unrestricted access to the internet and any file on your computer"
            }
        }
    }

    /// When Codex asks.
    pub(super) const fn approval(self) -> AskForApproval {
        match self {
            Self::ReadOnly | Self::WorkspaceWrite | Self::Agent => AskForApproval::OnRequest,
            Self::AgentFullAccess => AskForApproval::Never,
        }
    }

    /// Who reviews what it asks: a person, or Codex's own reviewer first.
    pub(super) const fn reviewer(self) -> ApprovalsReviewer {
        match self {
            Self::Agent => ApprovalsReviewer::AutoReview,
            Self::ReadOnly | Self::WorkspaceWrite | Self::AgentFullAccess => {
                ApprovalsReviewer::User
            }
        }
    }

    /// Its sandbox, as a thread's start names it.
    pub(super) const fn sandbox(self) -> SandboxMode {
        match self {
            Self::ReadOnly => SandboxMode::ReadOnly,
            Self::WorkspaceWrite | Self::Agent => SandboxMode::WorkspaceWrite,
            Self::AgentFullAccess => SandboxMode::DangerFullAccess,
        }
    }

    /// Its sandbox, as a turn's start names it.
    pub(super) const fn sandbox_policy(self) -> SandboxPolicy {
        match self {
            Self::ReadOnly => SandboxPolicy::ReadOnly {
                network_access: false,
            },
            Self::WorkspaceWrite | Self::Agent => SandboxPolicy::WorkspaceWrite {
                network_access: false,
            },
            Self::AgentFullAccess => SandboxPolicy::DangerFullAccess,
        }
    }

    /// The option, with this mode current.
    pub(super) fn option(self, policy: CodexVersionPolicy) -> SessionOption {
        SessionOption {
            id: OptionId::new(OPTION),
            name: "Mode".to_owned(),
            description: None,
            category: OptionCategory::Mode,
            kind: OptionKind::Select {
                current: OptionValueId::new(self.id()),
                values: Self::offered(policy)
                    .map(|mode| OptionValue {
                        id: OptionValueId::new(mode.id()),
                        name: mode.name().to_owned(),
                        description: Some(mode.description().to_owned()),
                        group: None,
                    })
                    .collect(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only full access sets `never` or `danger-full-access`.
    #[test]
    fn only_full_access_never_asks_or_leaves_the_sandbox() {
        for mode in Mode::ALL {
            let full = mode == Mode::AgentFullAccess;
            assert_eq!(mode.approval() == AskForApproval::Never, full, "{mode:?}");
            assert_eq!(
                mode.sandbox() == SandboxMode::DangerFullAccess,
                full,
                "{mode:?}"
            );
            assert_eq!(
                mode.sandbox_policy() == SandboxPolicy::DangerFullAccess,
                full,
                "{mode:?}"
            );
            assert_eq!(Mode::named(mode.id()), Some(mode));
        }
        assert_eq!(Mode::named("full"), None);
    }

    /// Only the mode option's own values are taken.
    #[test]
    fn a_choice_names_the_option_and_one_of_its_values() {
        let choose = |id: &str, value: ChoiceValue| OptionChoice {
            id: OptionId::new(id),
            value,
        };
        let select = |value: &str| ChoiceValue::Select {
            value: OptionValueId::new(value),
        };
        assert_eq!(
            Mode::chosen(
                &choose(OPTION, select("read-only")),
                CodexVersionPolicy::default()
            )
            .ok(),
            Some(Mode::ReadOnly)
        );
        assert!(matches!(
            Mode::chosen(
                &choose("model", select("read-only")),
                CodexVersionPolicy::default()
            ),
            Err(OptionError::Unknown { .. })
        ));
        assert!(matches!(
            Mode::chosen(
                &choose(OPTION, select("full")),
                CodexVersionPolicy::default()
            ),
            Err(OptionError::NotOffered { .. })
        ));
        assert!(matches!(
            Mode::chosen(
                &choose(OPTION, ChoiceValue::Toggle { on: true }),
                CodexVersionPolicy::default()
            ),
            Err(OptionError::WrongKind { .. })
        ));
    }
}
