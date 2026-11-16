//! Client commands are parsed before either immediate or queued provider sends.
//! Exact, attachment-free matching prevents swallowing a user's literal prompt.

/// Built-in composer operations are owned by the client, not the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientCommand {
    /// Archive and close the current session.
    Exit,
    /// Archive and retarget the tab to an equivalent new draft.
    Clear,
}

/// Whether context accompanies a prompt changes command interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// A single command token may be executed by the client.
    TextOnly,
    /// A prompt containing attachments always reaches the provider.
    Attached,
}

impl ClientCommand {
    /// Reference source order, before plugin and provider commands.
    pub const ALL: [Self; 2] = [Self::Exit, Self::Clear];

    /// Canonical spelling without the slash, shared by menu and execution.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Exit => "exit",
            Self::Clear => "clear",
        }
    }

    /// Aliases reserve their names when plugin and provider lists merge.
    #[must_use]
    pub const fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::Exit => &["quit", "q"],
            Self::Clear => &["new"],
        }
    }

    /// Only a complete reference token, without context, runs locally.
    #[must_use]
    pub fn parse(text: &str, context: Context) -> Option<Self> {
        if context == Context::Attached {
            return None;
        }
        let name = text
            .trim_matches(crate::matcher::whitespace)
            .strip_prefix('/')?;
        Self::ALL
            .into_iter()
            .find(|command| name == command.name() || command.aliases().contains(&name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_commands_take_exact_aliases_before_both_send_paths() {
        for (token, expected) in [
            ("/exit", ClientCommand::Exit),
            ("/quit", ClientCommand::Exit),
            ("/q", ClientCommand::Exit),
            ("/clear", ClientCommand::Clear),
            ("/new", ClientCommand::Clear),
        ] {
            assert_eq!(
                ClientCommand::parse(token, Context::TextOnly),
                Some(expected)
            );
            assert_eq!(
                ClientCommand::parse(&format!("  {token} \n"), Context::TextOnly),
                Some(expected)
            );
            assert_eq!(ClientCommand::parse(token, Context::Attached), None);
        }
    }

    #[test]
    fn arguments_case_and_prefixes_remain_provider_prompts() {
        for text in [
            "/exit now",
            "/clear\nplease",
            "/Exit",
            "hello /q",
            "/",
            "//exit",
            "/ex",
            "/q/x",
        ] {
            assert_eq!(
                ClientCommand::parse(text, Context::TextOnly),
                None,
                "{text}"
            );
        }
    }
    #[test]
    fn js_whitespace_does_not_turn_a_nel_suffixed_prompt_into_an_archive() {
        assert_eq!(
            ClientCommand::parse("\u{feff}/exit\u{feff}", Context::TextOnly),
            Some(ClientCommand::Exit)
        );
        for text in ["\u{0085}/exit", "/exit\u{0085}"] {
            assert_eq!(ClientCommand::parse(text, Context::TextOnly), None);
        }
    }
}
