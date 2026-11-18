//! Decode terminal entries once; approval hashing preserves their file order.
//! No terminal process can start through this parser.

use super::{MAX_COMMANDS, ProjectFileError, scripts};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use vessel_proto::workspace::{FileTerminal, trim_project_text};

/// Untrusted TOML stays here until its command and title have parsed.
#[derive(Deserialize)]
pub(super) struct RawTerminal {
    name: Option<String>,
    command: Option<String>,
}

/// Refuse the complete list rather than approving a silently shortened one.
pub(super) fn parse(raw: Vec<RawTerminal>) -> Result<Vec<FileTerminal>, ProjectFileError> {
    if raw.len() > MAX_COMMANDS {
        return Err(ProjectFileError::TooManyTerminals);
    }
    raw.into_iter().map(one).collect()
}

fn one(raw: RawTerminal) -> Result<FileTerminal, ProjectFileError> {
    let command = raw
        .command
        .ok_or(ProjectFileError::TerminalMissingCommand)?;
    let command = trim_project_text(&command)
        .parse()
        .map_err(ProjectFileError::TerminalCommand)?;
    let name = raw
        .name
        .as_deref()
        .map(trim_project_text)
        .filter(|name| !name.is_empty())
        .map(str::parse)
        .transpose()
        .map_err(ProjectFileError::TerminalName)?;
    Ok(FileTerminal { name, command })
}

/// Order is executable behaviour, so the approval must cover it.
pub(super) fn digest(hash: &mut Sha256, terminals: &[FileTerminal]) {
    // usize fits in u64 on supported targets; use the checked conversion
    // consistently with setup's length-prefixed encoding.
    hash.update(
        u64::try_from(terminals.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for terminal in terminals {
        let name = terminal.name.as_ref().map_or("", |name| name.as_str());
        scripts::field(hash, name.as_bytes());
        scripts::field(hash, terminal.command.as_str().as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_file::ProjectFile;

    #[test]
    fn fixture_keeps_order_and_trims_names() {
        let file =
            ProjectFile::parse(include_bytes!("../../tests/fixtures/file-terminals.toml")).unwrap();
        assert_eq!(file.terminals.len(), 2);
        assert_eq!(file.terminals[0].name.as_ref().unwrap().as_str(), "logs");
        assert_eq!(file.terminals[0].command.as_str(), "tail -f dev.log");
        assert_eq!(file.terminals[1].name, None);
        assert_eq!(file.terminals[1].command.as_str(), "echo ready");
        assert!(!file.is_empty());
    }

    #[test]
    fn terminal_text_uses_javascript_trimming_before_typed_validation() {
        let text =
            "[[worktree.terminals]]\nname = '\u{feff}'\ncommand = '\u{feff} echo ready \u{feff}'\n";
        let file = ProjectFile::parse(text.as_bytes()).unwrap();
        assert_eq!(file.terminals[0].name, None);
        assert_eq!(file.terminals[0].command.as_str(), "echo ready");
        let text = "[[worktree.terminals]]\ncommand = '\u{0085}echo ready\u{0085}'\n";
        let file = ProjectFile::parse(text.as_bytes()).unwrap();
        assert_eq!(
            file.terminals[0].command.as_str(),
            "\u{0085}echo ready\u{0085}"
        );
    }

    #[test]
    fn invalid_entry_refuses_every_command_in_the_file() {
        for bad in [
            "[[worktree.terminals]]\nname = 'missing'",
            "[[worktree.terminals]]\ncommand = ''",
            "[[worktree.terminals]]\ncommand = 1",
            "[worktree]\nterminals = ['not an entry']",
            "[[worktree.terminals]]\nname = 1\ncommand = 'x'",
            "[[worktree.terminals]]\nname = \"bad\\nname\"\ncommand = 'x'",
            "[[worktree.terminals]]\ncommand = \"bad\\u0000command\"",
        ] {
            assert!(ProjectFile::parse(bad.as_bytes()).is_err());
        }
        let too_long = format!(
            "[[worktree.terminals]]\nname = '{}'\ncommand = 'x'",
            "n".repeat(vessel_proto::SessionTitle::MAX_CHARS.saturating_add(1))
        );
        assert!(matches!(
            ProjectFile::parse(too_long.as_bytes()),
            Err(ProjectFileError::TerminalName(_))
        ));
        let command = format!("[[worktree.terminals]]\ncommand = '{}'", "x".repeat(4097));
        assert!(matches!(
            ProjectFile::parse(command.as_bytes()),
            Err(ProjectFileError::TerminalCommand(_))
        ));
        let entry = "[[worktree.terminals]]\ncommand = 'x'\n";
        assert!(ProjectFile::parse(entry.repeat(32).as_bytes()).is_ok());
        assert!(matches!(
            ProjectFile::parse(entry.repeat(33).as_bytes()),
            Err(ProjectFileError::TooManyTerminals)
        ));
    }

    #[test]
    fn digest_pins_names_commands_and_order_but_not_ignored_keys() {
        let digest = |text: &str| ProjectFile::parse(text.as_bytes()).unwrap().digest();
        let first = "[[worktree.terminals]]\nname = 'logs'\ncommand = 'tail -f dev.log'\n";
        let second = "[[worktree.terminals]]\ncommand = 'echo ready'\n";
        let both = format!("{first}{second}");
        assert_ne!(digest(&both), digest(&format!("{second}{first}")));
        assert_ne!(digest(&both), digest(&both.replace("logs", "output")));
        assert_ne!(
            digest(&both),
            digest(&both.replace("echo ready", "echo done"))
        );
        assert_eq!(digest(&both), digest(&format!("{both}note = 'ignored'")));
    }
    #[test]
    fn adr_encoding_preserves_old_approvals_and_pins_version_three() {
        let cases = [
            (
                "[worktree]\nsetup = 'ci'",
                "426b1da6a77b68d966ab4523ae8b262bbb68a9fae851842e329e552dded0dda9",
            ),
            (
                "[worktree]\nsetup = 'ci'\n[scripts.t]\ncommand = 'x'",
                "b5cea5d907b243cfda8ede7a045ab45a67a0a9558dd1f1c20e518dacd1e8fcd0",
            ),
            (
                "[worktree]\nsetup = 'ci'\n[scripts.t]\ncommand = 'x'\n[[worktree.terminals]]\nname = 'logs'\ncommand = 'tail -f dev.log'",
                "1289bdd68f22fa1bd7139a0d8020b6caf23650c1a4726947a8b935aa41d83c66",
            ),
        ];
        for (text, expected) in cases {
            assert_eq!(
                ProjectFile::parse(text.as_bytes())
                    .unwrap()
                    .digest()
                    .to_string(),
                expected
            );
        }
    }
}
