//! Transient system-context transport capability. Argv support alone is not
//! enough: replacement and restoration must both be proved before support is on.

use crate::DirectProtocol;

/// Whether a transport has a tested transient instruction-and-restore contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPromptSupport {
    /// Native transport can replace and restore system context without user input.
    Supported,
    /// A launch requesting the override must fail before spawning the provider.
    Unsupported,
}

/// Capability beside the harness launch policy. None is standard ACP, which
/// defines no system-prompt override. Native drivers remain unsupported until
/// both setting and restoring their native instructions have protocol evidence.
#[must_use]
pub const fn system_prompt_support(protocol: Option<DirectProtocol>) -> SystemPromptSupport {
    match protocol {
        None
        | Some(
            DirectProtocol::CodexAppServer
            | DirectProtocol::ClaudeStream
            | DirectProtocol::OpencodeHttp,
        ) => SystemPromptSupport::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_driver_advertises_support_from_schema_or_argv_knowledge_alone() {
        for protocol in [
            None,
            Some(DirectProtocol::CodexAppServer),
            Some(DirectProtocol::ClaudeStream),
            Some(DirectProtocol::OpencodeHttp),
        ] {
            assert_eq!(
                system_prompt_support(protocol),
                SystemPromptSupport::Unsupported
            );
        }
    }
}
