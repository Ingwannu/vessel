//! Transient system instructions, never persistent session input or argv.
//! Transport support is a separate explicit capability checked before spawning.

use std::{fmt, str::FromStr};
use vessel_proto::automation::{InvalidAutomationText, PromptText};

/// Complete launch-only Vessel supplemental instruction layer, preserving the
/// vendor-native base. Callers compose this value from known captured context;
/// an unavailable capture must not be guessed. Deliberately has no Serialize, Deserialize
/// or Display implementation: only a proven transport may expose its contents.
#[derive(Clone, PartialEq, Eq)]
pub struct SystemPromptOverride(PromptText);

impl SystemPromptOverride {
    /// Reuse an already validated prompt without reparsing or allocating.
    #[must_use]
    pub const fn from_prompt(prompt: PromptText) -> Self {
        Self(prompt)
    }

    /// For a native system-instruction transport only; never use as a user
    /// prompt, environment variable, command argument or diagnostic.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for SystemPromptOverride {
    type Err = InvalidAutomationText;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self)
    }
}

impl fmt::Debug for SystemPromptOverride {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SystemPromptOverride(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::automation::AutomationText;

    #[test]
    fn the_existing_transport_bound_and_control_rules_apply_without_echoing_text() {
        let text = "synthetic instruction\nnext line";
        let parsed: SystemPromptOverride = text.parse().unwrap();
        assert_eq!(parsed.as_str(), text);
        assert!(!format!("{parsed:?}").contains(text));
        assert_eq!(
            SystemPromptOverride::from_prompt(text.parse().unwrap()),
            parsed
        );
        assert!(
            "x".repeat(AutomationText::PROMPT_MAX)
                .parse::<SystemPromptOverride>()
                .is_ok()
        );
        assert!(
            "x".repeat(AutomationText::PROMPT_MAX + 1)
                .parse::<SystemPromptOverride>()
                .is_err()
        );
        for invalid in ["", "  \n", "private\0value", "private\u{85}value"] {
            let error = invalid.parse::<SystemPromptOverride>().unwrap_err();
            assert!(!error.to_string().contains("private"));
        }
    }

    #[test]
    fn unsupported_transports_refuse_before_looking_for_or_spawning_a_program() {
        use crate::{AgentError, AgentProcess, AgentSetup, Driver, LaunchRefusal};
        use vessel_proto::ResolvedMcpServers;
        let command = vessel_proc::CommandSpec::new("vessel-no-such-system-context-provider");
        for driver in [
            Driver::Acp,
            Driver::CodexAppServer,
            Driver::ClaudeStream,
            Driver::OpencodeHttp,
        ] {
            let setup = AgentSetup {
                system_prompt: Some("synthetic private instruction".parse().unwrap()),
                cwd: std::env::temp_dir(),
                model: None,
                resume: None,
                fork: None,
                options: Vec::new(),
                mcp_servers: ResolvedMcpServers::new(),
                handshake_timeout: crate::DEFAULT_HANDSHAKE_TIMEOUT,
                questions: false,
                private_dir: None,
            };
            assert!(!format!("{setup:?}").contains("synthetic private instruction"));
            let error = AgentProcess::spawn_with(driver, &command, setup).unwrap_err();
            assert!(
                matches!(error, AgentError::Refused(LaunchRefusal::SystemPromptUnsupported(refused)) if refused == driver)
            );
            assert!(!error.to_string().contains("synthetic private instruction"));
        }
    }
}
