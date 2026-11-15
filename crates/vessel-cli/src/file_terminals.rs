//! The approval's terminal commands must be visible before a person accepts it.
use std::fmt::Write as _;
use vessel_proto::workspace::FileTerminal;

/// Expose every approved command rather than accepting a digest the person cannot inspect.
pub(crate) fn text(terminals: &[FileTerminal]) -> String {
    let mut text = String::new();
    for terminal in terminals {
        let name = terminal.name.as_ref().map_or("", |name| name.as_str());
        let _written = writeln!(text, "terminal {name}: {}", terminal.command.as_str());
    }
    text
}

#[cfg(test)]
mod tests {
    #[test]
    fn approval_shows_named_and_unnamed_commands_in_file_order() {
        use vessel_proto::workspace::FileTerminal;
        let terminals = [
            FileTerminal {
                name: Some("logs".parse().unwrap()),
                command: "tail -f dev.log".parse().unwrap(),
            },
            FileTerminal {
                name: None,
                command: "echo ready".parse().unwrap(),
            },
        ];
        assert_eq!(
            super::text(&terminals),
            "terminal logs: tail -f dev.log\nterminal : echo ready\n"
        );
    }
}
