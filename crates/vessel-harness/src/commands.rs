//! Provider fallbacks belong beside the harness descriptor, never in the UI.

use vessel_proto::{AgentCommand, HarnessRef};

/// Fill only absent kinds using the provider's documented reference fallback.
/// Added agents and providers without one retain their reported metadata exactly.
pub fn classify_commands(harness: &HarnessRef, commands: &mut [AgentCommand]) {
    let Some(classify) = harness
        .known()
        .and_then(|kind| crate::harnesses::descriptor(kind).command_kind)
    else {
        return;
    };
    for command in commands {
        if command.kind.is_none() {
            command.kind = Some(classify(&command.name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::HarnessKind;
    use vessel_proto::composer_commands::CommandKind;

    fn command(name: &str, kind: Option<CommandKind>) -> AgentCommand {
        AgentCommand {
            name: name.parse().unwrap(),
            description: String::new(),
            hint: None,
            kind,
            native_command: None,
        }
    }

    #[test]
    fn fallback_is_provider_owned_and_never_replaces_explicit_metadata() {
        let mut commands = vec![
            command("compact", None),
            command("review", None),
            command("usage", Some(CommandKind::Skill)),
            command("future", Some(CommandKind::Unknown)),
        ];
        classify_commands(&HarnessKind::Codex.into(), &mut commands);
        assert_eq!(commands[0].kind, None);
        assert_eq!(commands[1].kind, None);
        classify_commands(&HarnessKind::ClaudeCode.into(), &mut commands);
        assert_eq!(commands[0].kind, Some(CommandKind::Command));
        assert_eq!(commands[1].kind, Some(CommandKind::Skill));
        assert_eq!(commands[2].kind, Some(CommandKind::Skill));
        assert_eq!(commands[3].kind, Some(CommandKind::Unknown));
        let mut added = vec![command("review", None)];
        classify_commands(
            &"custom-agent"
                .parse::<vessel_proto::AgentName>()
                .unwrap()
                .into(),
            &mut added,
        );
        assert_eq!(added[0].kind, None);
    }
}
