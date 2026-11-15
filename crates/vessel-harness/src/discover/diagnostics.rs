//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
use crate::{
    AddedAgent, Lookup,
    harnesses::{Structured, descriptor},
};
use vessel_proto::{
    HarnessKind,
    diagnostics::{DiagnosticAgentRefusal, DiagnosticAvailability, DiagnosticPresence},
};

/// Metadata facts do not claim sign-in, responsiveness or a successful turn.
#[derive(Debug)]
pub struct AgentMetadata {
    /// Whether the configured lookup found the program.
    pub executable: DiagnosticPresence,
    /// Whether the required executable/adapter is present.
    pub structured: DiagnosticAvailability,
    /// Whether terminal mode exists and its program is present.
    pub terminal: DiagnosticAvailability,
}
fn missing(reason: DiagnosticAgentRefusal) -> DiagnosticAvailability {
    DiagnosticAvailability::Unavailable { reason }
}
/// Inspect a builtin's metadata without running any program. Blocks on
/// filesystem lookup and must be called from a blocking context.
#[must_use]
pub fn builtin_metadata(kind: HarnessKind, lookup: &Lookup) -> AgentMetadata {
    let descriptor = descriptor(kind);
    let found = lookup.find(descriptor.binary).is_some();
    let structured = if found {
        match &descriptor.structured {
            Structured::Native { .. } => DiagnosticAvailability::Available,
            Structured::Adapter { binary, .. } => {
                if lookup.find(binary).is_some() {
                    DiagnosticAvailability::Available
                } else {
                    missing(DiagnosticAgentRefusal::MissingAdapter)
                }
            }
        }
    } else {
        missing(DiagnosticAgentRefusal::MissingExecutable)
    };
    AgentMetadata {
        executable: if found {
            DiagnosticPresence::Found
        } else {
            DiagnosticPresence::Missing
        },
        structured,
        terminal: if found {
            DiagnosticAvailability::Available
        } else {
            missing(DiagnosticAgentRefusal::MissingExecutable)
        },
    }
}
/// Inspect an added agent's written program/search path without executing it.
/// An added agent has no terminal mode, as ordinary discovery already states.
#[must_use]
pub fn added_metadata(agent: &AddedAgent, lookup: &Lookup) -> AgentMetadata {
    let found = crate::discover::found(agent, lookup).is_some();
    AgentMetadata {
        executable: if found {
            DiagnosticPresence::Found
        } else {
            DiagnosticPresence::Missing
        },
        structured: if found {
            DiagnosticAvailability::Available
        } else {
            missing(DiagnosticAgentRefusal::MissingExecutable)
        },
        terminal: missing(DiagnosticAgentRefusal::Unsupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_programs_have_exact_metadata_refusals_without_probing() {
        let temp = tempfile::tempdir().unwrap();
        let lookup = Lookup::new([temp.path().to_owned()]);
        for kind in HarnessKind::ALL {
            let facts = builtin_metadata(kind, &lookup);
            assert_eq!(facts.executable, DiagnosticPresence::Missing);
            assert_eq!(
                facts.structured,
                missing(DiagnosticAgentRefusal::MissingExecutable)
            );
            assert_eq!(
                facts.terminal,
                missing(DiagnosticAgentRefusal::MissingExecutable)
            );
        }
    }
    #[cfg(unix)]
    fn marker(path: &std::path::Path) -> std::path::PathBuf {
        let mut value = path.as_os_str().to_owned();
        value.push(".invocations");
        std::path::PathBuf::from(value)
    }
    #[cfg(unix)]
    #[test]
    fn adapter_presence_is_distinct_and_metadata_never_invokes_the_files() {
        use crate::fakes::{Role, install};
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        let fake = temp.path().join("metadata-canary");
        vessel_testkit::write_executable(
            &fake,
            "#!/bin/sh\nprintf 'invoked\\n' >> \"$0.invocations\"\nexit 97\n",
        )
        .unwrap();
        let terminal = install(&bin, HarnessKind::ClaudeCode, Role::Terminal, &fake).unwrap();
        let lookup = Lookup::new([bin.clone()]);
        let before = builtin_metadata(HarnessKind::ClaudeCode, &lookup);
        assert_eq!(before.executable, DiagnosticPresence::Found);
        assert_eq!(
            before.structured,
            missing(DiagnosticAgentRefusal::MissingAdapter)
        );
        assert_eq!(before.terminal, DiagnosticAvailability::Available);
        let adapter = install(&bin, HarnessKind::ClaudeCode, Role::Structured, &fake).unwrap();
        for _ in 0..2 {
            let after = builtin_metadata(HarnessKind::ClaudeCode, &lookup);
            assert_eq!(after.structured, DiagnosticAvailability::Available);
        }
        assert!(!marker(&terminal).exists());
        assert!(!marker(&adapter).exists());
        assert!(!marker(&fake).exists());
    }
    #[cfg(unix)]
    #[test]
    fn added_agent_metadata_omits_definition_values_and_never_invokes_it() {
        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("metadata-added");
        vessel_testkit::write_executable(
            &program,
            "#!/bin/sh\nprintf 'invoked\\n' >> \"$0.invocations\"\nexit 97\n",
        )
        .unwrap();
        let lookup = Lookup::new([temp.path().to_owned()]);
        let private = "private-diagnostic-source-canary";
        let text = format!(
            "[agents.private-agent]\nprogram = \"metadata-added\"\nlabel = \"{private}\"\nargs = [\"{private}\"]\nenv = {{ MODE = \"{private}\" }}\n"
        );
        let configured = crate::parse_agents(&text, None).unwrap();
        let name = "private-agent".parse().unwrap();
        let agent = configured.get(&name).unwrap();
        let facts = added_metadata(agent, &lookup);
        assert_eq!(facts.executable, DiagnosticPresence::Found);
        assert_eq!(facts.structured, DiagnosticAvailability::Available);
        assert_eq!(facts.terminal, missing(DiagnosticAgentRefusal::Unsupported));
        assert!(!format!("{facts:?}").contains(private));
        assert!(!marker(&program).exists());
    }
}
