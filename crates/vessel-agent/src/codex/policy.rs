//! One projection of observed Codex versions. Runtime mode gating and launch
//! feature decisions share the harness's existing parser, never another probe.

use vessel_harness::Version;
use vessel_proto::agents::AgentVersion;

/// Native version floors, not permission or process-ownership authority.
/// A goals-capable version alone does not prove `--enable goals` was launched.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CodexVersionPolicy {
    version: Option<Version>,
}

impl CodexVersionPolicy {
    /// Project already-observed bounded metadata. This starts no process.
    #[must_use]
    pub fn observed(version: Option<&AgentVersion>) -> Self {
        Self {
            version: version.and_then(|version| Version::find(version.as_str())),
        }
    }

    /// Paseo's auto-review floor (0.115.0). Unknown versions cannot enable it.
    #[must_use]
    pub fn auto_review(self) -> bool {
        self.version
            .is_some_and(|version| version >= Version::new(0, 115, 0))
    }

    /// Paseo's goals version floor (0.128.0), without claiming its launch flag.
    #[must_use]
    pub fn goals_version_supported(self) -> bool {
        self.version
            .is_some_and(|version| version >= Version::new(0, 128, 0))
    }
}

/// The bounded native version token, shared by live and prompt-free handshakes.
pub(super) fn version_of(user_agent: &str) -> Option<AgentVersion> {
    let first = user_agent.split_whitespace().next()?;
    let version = first.split_once('/').map_or(first, |(_, version)| version);
    version.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_version_floors_share_the_existing_parser_and_unknown_disables_features() {
        for (text, auto, goals) in [
            ("0.114.9", false, false),
            ("0.115.0", true, false),
            ("0.127.9", true, false),
            ("0.128.0", true, true),
            ("0.169.0-alpha", true, true),
            ("unknown", false, false),
        ] {
            let version = text.parse().unwrap();
            let policy = CodexVersionPolicy::observed(Some(&version));
            assert_eq!(policy.auto_review(), auto);
            assert_eq!(policy.goals_version_supported(), goals);
        }
        assert!(!CodexVersionPolicy::default().auto_review());
        assert!(!CodexVersionPolicy::default().goals_version_supported());
    }
}
