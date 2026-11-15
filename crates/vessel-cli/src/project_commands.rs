//! Approval text from the exact project-command snapshot a person may approve.
//! Complete entry patterns make a new approved field a required display decision.

use std::fmt::Write as _;

use vessel_proto::scripts::{ScriptEntry, ScriptEntryView, ScriptKind};
use vessel_proto::workspace::{ProjectCommands, ServicePorts, ServicePortsView};

/// Keep every approved command in view before the CLI submits its digest.
pub(crate) fn text(commands: &ProjectCommands) -> String {
    match commands {
        ProjectCommands::None => {
            "no commands (no vessel.toml, or nothing under [worktree], [scripts] or [services])\n"
                .to_owned()
        }
        ProjectCommands::Invalid { message } => format!("vessel.toml cannot be used: {message}\n"),
        ProjectCommands::Listed {
            setup,
            teardown,
            scripts,
            terminals,
            service_ports,
            digest,
            approved,
            last_approved,
        } => {
            let mut text = String::new();
            for (list, name) in [(setup, "setup"), (teardown, "teardown")] {
                for command in list {
                    let _written = writeln!(text, "{name}: {}", command.as_str());
                }
            }
            text.push_str(&crate::file_terminals::text(terminals));
            if let Some(ports) = service_ports {
                write_allocation(&mut text, ports);
            }
            write_scripts(&mut text, scripts);
            let _written = writeln!(
                text,
                "digest: {digest}\napproved: {}",
                if *approved { "yes" } else { "no" }
            );
            if let Some(last) = last_approved {
                let _written = writeln!(
                    text,
                    "last approved: {} ({}), {} setup, {} teardown, {} scripts and services",
                    last.digest,
                    last.approved_at.to_rfc3339(),
                    last.commands.setup.len(),
                    last.commands.teardown.len(),
                    last.commands.scripts.len()
                );
            }
            text
        }
    }
}

// Both approved choices remain visible, independently of execution precedence.
fn write_allocation(text: &mut String, ports: &ServicePorts) {
    let ServicePortsView { range, port_script } = ports.view();
    if let Some(range) = range {
        let _written = writeln!(text, "service port range: {range}");
    }
    if let Some(program) = port_script {
        let _written = writeln!(text, "service port script: {program}");
    }
}

// Borrow the output buffer so listing scripts adds no second command-string copy.
fn write_scripts(text: &mut String, scripts: &[ScriptEntry]) {
    for script in scripts {
        let ScriptEntryView {
            name,
            kind,
            command,
            dir,
            port,
        } = script.view();
        let _written = writeln!(
            text,
            "{} {}: {}{}{}",
            match kind {
                ScriptKind::Script => "script",
                ScriptKind::Service => "service",
            },
            name,
            command.as_str(),
            dir.map_or_else(String::new, |dir| format!("  (in {dir})")),
            port.map_or_else(String::new, |port| format!("  (port {port})"))
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::workspace::{CommandsDigest, ServicePort};

    fn listed(scripts: Vec<ScriptEntry>, service_ports: Option<ServicePorts>) -> ProjectCommands {
        ProjectCommands::Listed {
            setup: Vec::new(),
            teardown: Vec::new(),
            terminals: Vec::new(),
            scripts,
            service_ports,
            digest: CommandsDigest::from_sha256(std::array::from_fn(|_| 0xab)),
            approved: false,
            last_approved: None,
        }
    }

    #[test]
    fn declared_ports_are_visible_beside_the_exact_command_and_folder() {
        let commands = listed(
            vec![
                ScriptEntry::new(
                    "dev".parse().unwrap(),
                    ScriptKind::Service,
                    "serve-approval-port".parse().unwrap(),
                    Some("frontend".parse().unwrap()),
                    Some(ServicePort::try_from(3210_u16).unwrap()),
                )
                .unwrap(),
            ],
            None,
        );
        assert!(
            text(&commands)
                .contains("service dev: serve-approval-port  (in frontend)  (port 3210)\n")
        );
    }
    #[test]
    fn allocation_approval_shows_both_choices_even_when_the_program_wins() {
        let ports = ServicePorts::new(
            Some("7000-7009".parse().unwrap()),
            Some("tools/port-approval".parse().unwrap()),
        )
        .unwrap();
        let shown = text(&listed(Vec::new(), Some(ports)));
        assert!(shown.contains("service port range: 7000-7009\n"));
        assert!(shown.contains("service port script: tools/port-approval\n"));
    }
}
