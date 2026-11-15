//! Provider execution policy reuses the harness's existing executable/home and
//! direct-argument tables. Catalogue metadata never implies driver availability.

use crate::{DirectProtocol, direct, harnesses::descriptor};
use vessel_proto::{HarnessKind, providers::ProviderKind};

/// Which protocol an executable must speak after native arguments are appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderProtocol {
    /// Native ACP, including configured arbitrary ACP agents.
    Acp,
    /// One of the existing direct transports.
    Direct(DirectProtocol),
}

/// Inert vendor launch facts, without availability, lookup or credentials.
#[derive(Debug, Clone, Copy)]
pub struct ProviderTransport {
    /// Driver selection; the consumer must check actual build support.
    pub protocol: ProviderProtocol,
    /// None requires an explicit configured replacement command.
    pub program: Option<&'static str>,
    /// Protocol arguments always follow the configured executable prefix.
    pub arguments: &'static [&'static str],
    /// Explicit account home variable before configured overrides.
    pub home: Option<&'static str>,
}

/// Native transports currently described by Vessel. Missing Pi/OMP/plugin
/// transports are an explicit gap, never an ACP fallback or invented command.
#[must_use]
pub fn provider_transport(kind: ProviderKind) -> Option<ProviderTransport> {
    let known = match kind {
        ProviderKind::Claude => HarnessKind::ClaudeCode,
        ProviderKind::Codex => HarnessKind::Codex,
        ProviderKind::OpenCode => HarnessKind::OpenCode,
        ProviderKind::Copilot => {
            return Some(ProviderTransport {
                protocol: ProviderProtocol::Acp,
                program: Some("copilot"),
                arguments: &["--acp"],
                home: None,
            });
        }
        ProviderKind::Acp => {
            return Some(ProviderTransport {
                protocol: ProviderProtocol::Acp,
                program: None,
                arguments: &[],
                home: None,
            });
        }
        ProviderKind::Pi | ProviderKind::Omp | ProviderKind::Plugin => return None,
    };
    let native = direct(known)?;
    let descriptor = descriptor(known);
    Some(ProviderTransport {
        protocol: ProviderProtocol::Direct(native.protocol),
        program: Some(descriptor.binary),
        arguments: native.args,
        home: descriptor.home.variable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_provider_launch_facts_have_one_existing_owner() {
        for (provider, harness) in [
            (ProviderKind::Codex, HarnessKind::Codex),
            (ProviderKind::Claude, HarnessKind::ClaudeCode),
            (ProviderKind::OpenCode, HarnessKind::OpenCode),
        ] {
            let policy = provider_transport(provider).unwrap();
            let native = direct(harness).unwrap();
            assert_eq!(policy.arguments, native.args);
            assert_eq!(policy.protocol, ProviderProtocol::Direct(native.protocol));
            assert_eq!(policy.program, Some(descriptor(harness).binary));
        }
        assert!(provider_transport(ProviderKind::Pi).is_none());
        assert!(provider_transport(ProviderKind::Omp).is_none());
    }
}
