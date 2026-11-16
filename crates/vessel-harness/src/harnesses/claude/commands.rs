//! Paseo 0.15.4 Claude agent.ts:420-450 classifies its flat SDK list this way.
//! Keep the fallback at the provider edge: other agents cannot safely use it.

use vessel_proto::composer_commands::CommandKind;
use vessel_proto::conversation::CommandName;

pub(super) fn kind(name: &CommandName) -> CommandKind {
    if matches!(
        name.as_str(),
        "clear"
            | "compact"
            | "context"
            | "debug"
            | "extra-usage"
            | "heapdump"
            | "init"
            | "loop"
            | "schedule"
            | "usage"
    ) {
        CommandKind::Command
    } else {
        CommandKind::Skill
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_root_controls_are_excluded_from_inline_skills() {
        for name in [
            "clear",
            "compact",
            "context",
            "debug",
            "extra-usage",
            "heapdump",
            "init",
            "loop",
            "schedule",
            "usage",
        ] {
            assert_eq!(kind(&name.parse().unwrap()), CommandKind::Command);
        }
        for name in ["review", "team:review", "rewind"] {
            assert_eq!(kind(&name.parse().unwrap()), CommandKind::Skill);
        }
    }
}
