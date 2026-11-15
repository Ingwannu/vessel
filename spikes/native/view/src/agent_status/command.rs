//! Copyable added-agent launch arguments, without guessing a remote shell.
//!
//! [Decision Log]
//! - Purpose: copy a long launch command without turning an argument’s spaces,
//!   dollar signs, quotes or newlines into shell syntax.
//! - Constraints: the node reports expanded argv and OS, but no shell or secret
//!   values. The CLI’s space join is for display, not safe to execute.
//! - Alternatives: join with spaces/newlines, require a shell-escaping crate,
//!   or show each argv value separately on every platform.
//! - Choice: single-quote every value for known Unix machines, escaping an
//!   embedded apostrophe by closing the quote and quoting that character.
//!   Other machines keep separately labelled values; no Windows shell is guessed.
//! - Cost: this is program/arguments only, explicitly excluding environment;
//!   Windows has no combined copy action until the node names its shell.
use vessel_proto::agents::AddedAgent;

/// How the machine’s argv may be copied, parsed once from its OS metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandShell {
    /// A command for sh, bash or zsh.
    Unix,
    /// Shell syntax is unknown; retain individual argv values.
    Unspecified,
}
impl CommandShell {
    /// Rust’s node OS name, not the client machine’s platform.
    #[must_use]
    pub fn for_os(os: &str) -> Self {
        match os {
            "linux" | "macos" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "illumos"
            | "solaris" | "android" => Self::Unix,
            _ => Self::Unspecified,
        }
    }
    /// The node expanded home-relative values before returning this definition.
    #[must_use]
    pub fn command(self, added: &AddedAgent) -> Option<String> {
        match self {
            Self::Unix => Some(
                std::iter::once(added.program.as_str())
                    .chain(added.args.iter().map(String::as_str))
                    .map(quote)
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            Self::Unspecified => None,
        }
    }
}
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoting_preserves_each_argument_instead_of_creating_shell_syntax() {
        for (value, expected) in [
            ("", "''"),
            ("word", "'word'"),
            ("two words", "'two words'"),
            ("it's", "'it'\"'\"'s'"),
            ("$(touch never); $HOME", "'$(touch never); $HOME'"),
            ("한글\nnext", "'한글\nnext'"),
        ] {
            assert_eq!(quote(value), expected);
        }
        let added: AddedAgent = serde_json::from_value(serde_json::json!({"program":"/opt/agent's folder/agent","args":["","$HOME","a b"],"source":{"from":"command"}})).unwrap();
        assert_eq!(
            CommandShell::Unix.command(&added).as_deref(),
            Some("'/opt/agent'\"'\"'s folder/agent' '' '$HOME' 'a b'")
        );
        assert_eq!(CommandShell::Unspecified.command(&added), None);
    }
    #[test]
    fn the_remote_os_determines_whether_a_shell_command_can_be_shown() {
        for os in [
            "linux",
            "macos",
            "freebsd",
            "openbsd",
            "netbsd",
            "dragonfly",
            "illumos",
            "solaris",
            "android",
        ] {
            assert_eq!(CommandShell::for_os(os), CommandShell::Unix);
        }
        for os in ["windows", "", "other"] {
            assert_eq!(CommandShell::for_os(os), CommandShell::Unspecified);
        }
    }
}
