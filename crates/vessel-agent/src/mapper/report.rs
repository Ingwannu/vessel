//! What an agent reports about its session besides the timeline (protocol
//! 10): the commands it offers, how full its context is, and a turn's
//! tokens. Pure conversions, bounded on the way in.
//!
//! An item that does not fit Vessel's types (a command name with white
//! space, a cost in a unit that is not one word) is left out with a trace,
//! never passed on unchecked: the rest of the report still counts.

use agent_client_protocol::v1::schema as acp;
use vessel_proto::TokenCount;
use vessel_proto::conversation::{CommandName, CostUnit};
use vessel_proto::conversation::{Cost, FiniteNumber};
use vessel_proto::{AgentCommand, ContextUsage, TurnUsage};

/// The commands of an `available_commands_update`, at most
/// [`AgentCommand::MAX_COMMANDS`], each cut to its bounds.
pub(crate) fn commands(update: acp::AvailableCommandsUpdate) -> Vec<AgentCommand> {
    let offered = update.available_commands.len();
    if offered > AgentCommand::MAX_COMMANDS {
        tracing::warn!(
            offered,
            kept = AgentCommand::MAX_COMMANDS,
            "the agent offers more commands than a session keeps"
        );
    }
    update
        .available_commands
        .into_iter()
        .take(AgentCommand::MAX_COMMANDS)
        .filter_map(command)
        .collect()
}

fn command(command: acp::AvailableCommand) -> Option<AgentCommand> {
    // Some agents write the slash themselves; the name is what follows it.
    let name = command.name.strip_prefix('/').unwrap_or(&command.name);
    let name = match name.parse::<CommandName>() {
        Ok(name) => name,
        Err(error) => {
            tracing::debug!(name = %command.name, %error, "agent command left out");
            return None;
        }
    };
    let hint = match command.input {
        Some(acp::AvailableCommandInput::Unstructured(input)) => {
            Some(cut(input.hint, AgentCommand::MAX_HINT))
        }
        Some(_) | None => None,
    };
    Some(AgentCommand {
        name,
        description: cut(command.description, AgentCommand::MAX_DESCRIPTION),
        hint: hint.filter(|hint| !hint.is_empty()),
    })
}

/// A `usage_update` as the session's context figure. A cost whose amount
/// is not finite, or whose unit is not one word, is left out.
pub(crate) fn usage(update: acp::UsageUpdate) -> ContextUsage {
    let cost = update.cost.and_then(|cost| {
        let amount = FiniteNumber::new(cost.amount)?;
        let unit = cost.currency.parse::<CostUnit>().ok()?;
        Some(Cost { amount, unit })
    });
    ContextUsage {
        observation: vessel_proto::conversation::UsageObservation::Current,
        used: TokenCount::new(update.used),
        size: TokenCount::new(update.size),
        cost,
    }
}

/// A turn's tokens from the prompt's answer, when the agent gives them.
pub(crate) fn turn_usage(usage: &acp::Usage) -> TurnUsage {
    TurnUsage {
        total: TokenCount::new(usage.total_tokens),
        input: TokenCount::new(usage.input_tokens),
        output: TokenCount::new(usage.output_tokens),
        thought: usage.thought_tokens.map(TokenCount::new),
        cached_read: usage.cached_read_tokens.map(TokenCount::new),
        cached_write: usage.cached_write_tokens.map(TokenCount::new),
    }
}

/// `text` cut to at most `max` bytes, at a character boundary.
pub(super) fn cut(mut text: String, max: usize) -> String {
    if text.len() > max {
        let end = text.floor_char_boundary(max);
        text.truncate(end);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_checked_cut_and_bounded() {
        // Trimmed from `claude-agent-acp` 0.100's update.
        let update: acp::AvailableCommandsUpdate = serde_json::from_str(
            r#"{"availableCommands":[
                {"name":"compact","description":"Clear history but keep a summary",
                 "input":{"hint":"<optional custom summarization instructions>"}},
                {"name":"/review","description":"Review a pull request"},
                {"name":"two words","description":"refused"}
            ]}"#,
        )
        .unwrap();
        let commands = commands(update);
        let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["compact", "review"]);
        assert_eq!(
            commands[0].hint.as_deref(),
            Some("<optional custom summarization instructions>")
        );
        assert_eq!(commands[1].hint, None);
        assert_eq!(cut("한글".to_owned(), 4), "한", "never inside a character");
    }

    #[test]
    fn usage_keeps_a_cost_only_when_it_is_one() {
        let update: acp::UsageUpdate = serde_json::from_str(
            r#"{"used":41200,"size":200000,"cost":{"amount":0.25,"currency":"USD"}}"#,
        )
        .unwrap();
        let usage = usage(update);
        assert_eq!(usage.used, TokenCount::new(41_200));
        assert_eq!(
            usage.cost.map(|cost| cost.unit.to_string()).as_deref(),
            Some("USD")
        );
        let odd: acp::UsageUpdate = serde_json::from_str(
            r#"{"used":1,"size":2,"cost":{"amount":1,"currency":"US dollars"}}"#,
        )
        .unwrap();
        assert_eq!(super::usage(odd).cost, None);
    }
}
