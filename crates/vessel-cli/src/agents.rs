//! `vessel agents …` (protocol 11, ADR 0032): what a node can run, and the
//! agents its owner adds by their command.

use std::fmt::Write as _;

use vessel_proto::ConfigValue;
use vessel_proto::agents::{
    AddAgent, AgentDefinition, AgentDetail, AgentInfo, AgentSource, AgentToAdd, AgentVariable,
    ListAgents, RemoveAgent,
};

use crate::args::{AddAgentArgs, AgentsCommand, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::io;

pub(crate) async fn agents(target: &Target, command: AgentsCommand) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    match command {
        AgentsCommand::List => {
            let agents = client.call(ListAgents {}).await?;
            out(&list(&agents)).await
        }
        AgentsCommand::Add(args) => {
            let request = add(args)?;
            let added = client.call(request).await?;
            out(&list(std::slice::from_ref(&added))).await
        }
        AgentsCommand::Remove { name } => {
            let _ack = client.call(RemoveAgent { name: name.clone() }).await?;
            out(&format!("removed {name}\n")).await
        }
    }
}

/// The request `vessel agents add` sends.
fn add(args: AddAgentArgs) -> Result<AddAgent, CliError> {
    let AddAgentArgs {
        name,
        label,
        env,
        secrets,
        inherit,
        mcp,
        replace,
        command,
    } = args;
    let mut words = command.into_iter();
    let program = words
        .next()
        .ok_or(CliError::Usage("give the agent's program after `--`"))?;
    let program = program.parse()?;
    let variables: Vec<AgentVariable> = env
        .into_iter()
        .map(|(name, value)| AgentVariable {
            name,
            value: ConfigValue::Text(value),
        })
        .chain(secrets.into_iter().map(|(name, secret)| AgentVariable {
            name,
            value: ConfigValue::Secret(secret),
        }))
        .collect();
    Ok(AddAgent {
        name,
        agent: AgentToAdd::Command {
            definition: AgentDefinition {
                label,
                program,
                args: words.collect(),
                env: variables,
                inherit,
                mcp,
            },
        },
        replace,
    })
}

/// Each agent on a line, an added one's command after it.
pub(crate) fn list(agents: &[AgentInfo]) -> String {
    let mut text = String::new();
    for agent in agents {
        let agent_ref = &agent.agent;
        match &agent.detail {
            AgentDetail::Known { .. } => {
                let _infallible = writeln!(text, "{agent_ref}");
            }
            AgentDetail::Added { added } => {
                let label = added
                    .label
                    .as_ref()
                    .map_or_else(String::new, |label| format!(" ({label})"));
                let command = std::iter::once(added.program.as_str())
                    .chain(added.args.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join(" ");
                let _infallible = writeln!(text, "{agent_ref}{label}: {command}");
                let names = |names: Vec<String>| names.join(", ");
                if !added.env.is_empty() {
                    let set = names(added.env.iter().map(ToString::to_string).collect());
                    let _infallible = writeln!(text, "  sets {set}");
                }
                for secret in &added.secrets {
                    let _infallible = writeln!(
                        text,
                        "  sets {} from secret {}",
                        secret.variable, secret.secret
                    );
                }
                if !added.inherit.is_empty() {
                    let passed = names(added.inherit.iter().map(ToString::to_string).collect());
                    let _infallible = writeln!(text, "  passes on {passed}");
                }
                if !added.mcp.is_empty() {
                    let servers = names(added.mcp.iter().map(ToString::to_string).collect());
                    let _infallible = writeln!(text, "  MCP servers {servers}");
                }
                if let AgentSource::Registry { id, version } = &added.source {
                    let _infallible = writeln!(text, "  from the ACP registry: {id} {version}");
                }
            }
        }
    }
    text
}

async fn out(text: &str) -> Result<(), CliError> {
    Ok(io::out(text.as_bytes()).await?)
}

#[cfg(test)]
mod tests {
    use vessel_proto::HarnessKind;
    use vessel_proto::agents::{AddedAgent, SecretEnv};

    use super::*;

    #[test]
    fn add_sends_text_and_secrets_and_list_shows_names_only() {
        let request = add(AddAgentArgs {
            name: "goose".parse().unwrap(),
            label: None,
            env: vec![("GOOSE_MODE".parse().unwrap(), "auto".to_owned())],
            secrets: vec![(
                "ANTHROPIC_API_KEY".parse().unwrap(),
                "GOOSE_KEY".parse().unwrap(),
            )],
            inherit: Vec::new(),
            mcp: Vec::new(),
            replace: false,
            command: vec!["goose".to_owned(), "acp".to_owned()],
        })
        .unwrap();
        let AgentToAdd::Command { definition } = &request.agent;
        assert_eq!(definition.program.as_str(), "goose");
        assert_eq!(definition.args, ["acp"]);
        assert_eq!(
            definition.env[1].value,
            ConfigValue::Secret("GOOSE_KEY".parse().unwrap())
        );

        let shown = list(&[
            AgentInfo {
                agent: HarnessKind::Codex.into(),
                detail: AgentDetail::Known {
                    installs: Vec::new(),
                    sign_in: vessel_proto::agents::SignInHelp::default(),
                },
            },
            AgentInfo {
                agent: request.name.clone().into(),
                detail: AgentDetail::Added {
                    added: AddedAgent {
                        label: Some("Goose".parse().unwrap()),
                        program: definition.program.clone(),
                        args: definition.args.clone(),
                        env: vec!["GOOSE_MODE".parse().unwrap()],
                        secrets: vec![SecretEnv {
                            variable: "ANTHROPIC_API_KEY".parse().unwrap(),
                            secret: "GOOSE_KEY".parse().unwrap(),
                        }],
                        inherit: Vec::new(),
                        mcp: Vec::new(),
                        source: AgentSource::Command,
                    },
                },
            },
        ]);
        assert_eq!(
            shown,
            "codex\ngoose (Goose): goose acp\n  sets GOOSE_MODE\n  sets ANTHROPIC_API_KEY \
             from secret GOOSE_KEY\n"
        );
        assert!(!shown.contains("auto"), "a value is never listed: {shown}");
    }
}
