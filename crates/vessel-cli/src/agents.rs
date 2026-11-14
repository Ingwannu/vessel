//! `vessel agents …` (protocol 11, ADR 0032): what a node can run, and the
//! agents its owner adds by their command.

use std::fmt::Write as _;

use vessel_client::NodeClient;
use vessel_proto::agents::{
    AddAgent, AgentDefinition, AgentDetail, AgentInfo, AgentReadiness, AgentSource, AgentToAdd,
    AgentVariable, Catalogue, CatalogueRun, GetAgentStatus, GetCatalogue, InstallAgent,
    InstallPart, InstallState, Installed, ListAgents, ReadInstall, RegistryId, RemoveAgent,
    SignedIn, UnknownReason,
};
use vessel_proto::{AgentName, ConfigValue, HarnessKind, McpServerName};

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
            let added = match args.from_registry.clone() {
                Some(id) => {
                    let AddAgentArgs {
                        name,
                        mcp,
                        replace,
                        yes,
                        ..
                    } = args;
                    add_from_registry(&client, name, id, mcp, replace, yes).await?
                }
                None if args.yes => {
                    return Err(CliError::Usage(
                        "--yes answers for a registry entry (--from-registry); a command is added as written",
                    ));
                }
                None => client.call(add(args)?).await?,
            };
            out(&list(std::slice::from_ref(&added))).await
        }
        AgentsCommand::Remove { name } => {
            let _ack = client.call(RemoveAgent { name: name.clone() }).await?;
            out(&format!("removed {name}\n")).await
        }
        AgentsCommand::Status {
            agent,
            profile,
            refresh,
        } => {
            let listed = client
                .call(GetAgentStatus {
                    agent,
                    profile,
                    refresh,
                })
                .await?;
            out(&readiness(&listed)).await
        }
        AgentsCommand::Install {
            agent,
            adapter,
            yes,
        } => install(&client, agent, adapter, yes).await,
        AgentsCommand::Catalogue { refresh } => {
            let catalogue = client.call(GetCatalogue { refresh }).await?;
            out(&catalogue_text(&catalogue)).await
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
        from_registry: _,
        yes: _,
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

/// `vessel agents status`: each agent and profile, one line each.
pub(crate) fn readiness(listed: &[AgentReadiness]) -> String {
    let mut text = String::new();
    for readiness in listed {
        let AgentReadiness {
            agent,
            profile,
            installed,
            signed_in,
            checked_at,
        } = readiness;
        let installed = match installed {
            Installed::Found { version, .. } => version.as_ref().map_or_else(
                || "installed".to_owned(),
                |version| format!("installed {version}"),
            ),
            Installed::Missing { program } => format!("not installed ({program} not found)"),
            Installed::TooOld { speaks, needs, .. } => {
                format!("speaks ACP {speaks}, older than Vessel's {needs}")
            }
            Installed::TooNew { speaks, needs, .. } => {
                format!("speaks ACP {speaks}, newer than Vessel's {needs}")
            }
            Installed::Unknown { reason } => format!("installed: unknown ({})", why(reason)),
        };
        let signed_in = match signed_in {
            SignedIn::Yes => "signed in".to_owned(),
            SignedIn::NotRequired => "no sign-in (a model route)".to_owned(),
            SignedIn::No { help } => {
                let mut line = "not signed in".to_owned();
                if let Some(command) = &help.command {
                    let _infallible = write!(line, "; sign in with `{command}`");
                }
                if let Some(home) = &help.home {
                    let _infallible = write!(line, " with the profile's {home}");
                }
                for method in &help.methods {
                    let _infallible = write!(line, "; it offers {}", method.name);
                }
                line
            }
            SignedIn::Unknown { reason } => format!("signed in: unknown ({})", why(reason)),
        };
        let _infallible = writeln!(
            text,
            "{agent} ({profile}): {installed}; {signed_in} (as of {})",
            checked_at.format("%H:%M:%S")
        );
    }
    text
}

fn why(reason: &UnknownReason) -> String {
    match reason {
        UnknownReason::NotInstalled => "not installed".to_owned(),
        UnknownReason::NotChecked => "not checked; --refresh checks".to_owned(),
        UnknownReason::InUse => "a session of this profile runs".to_owned(),
        UnknownReason::NotSaid => "its handshake did not say".to_owned(),
        UnknownReason::NoHome => "the profile's home is a relative path".to_owned(),
        UnknownReason::Unread => "its check answered in words Vessel does not read".to_owned(),
        UnknownReason::TimedOut { after } => {
            format!("no answer in {} s", after.as_duration().as_secs())
        }
        UnknownReason::Failed { step } => format!("the check failed at {step:?}"),
    }
}

/// `vessel agents catalogue`: each entry and what would run.
pub(crate) fn catalogue_text(catalogue: &Catalogue) -> String {
    let mut text = String::new();
    match catalogue.fetched_at {
        None => {
            let _infallible = writeln!(text, "not fetched yet; --refresh fetches the ACP registry");
        }
        Some(at) => {
            let _infallible = writeln!(
                text,
                "the ACP registry as fetched at {} ({} entries, {} skipped)",
                at.format("%Y-%m-%d %H:%M:%S UTC"),
                catalogue.entries.len(),
                catalogue.skipped
            );
        }
    }
    for entry in &catalogue.entries {
        let runs = match &entry.runs {
            CatalogueRun::Known { harness } => {
                format!("Vessel's own {harness}: `vessel agents install`")
            }
            CatalogueRun::Command {
                program,
                args,
                runner_found,
                ..
            } => {
                let command = std::iter::once(program.as_str())
                    .chain(args.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join(" ");
                if *runner_found {
                    format!("runs `{command}`")
                } else {
                    format!("runs `{command}` ({program} not found on the node)")
                }
            }
            CatalogueRun::Download { archive, .. } => {
                format!("an archive only ({archive}): install it, then add it by command")
            }
            CatalogueRun::Unavailable => "nothing for this platform".to_owned(),
        };
        let _infallible = writeln!(
            text,
            "{} {} ({}): {runs}",
            entry.id, entry.version, entry.name
        );
    }
    text
}

/// `vessel agents add <name> --from-registry <id>`: show what the node's
/// copy would run, ask, add.
async fn add_from_registry(
    client: &NodeClient,
    name: AgentName,
    id: RegistryId,
    mcp: Vec<McpServerName>,
    replace: bool,
    yes: bool,
) -> Result<AgentInfo, CliError> {
    let catalogue = client.call(GetCatalogue { refresh: false }).await?;
    let entry = catalogue
        .entries
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or(CliError::Usage(
            "the node's copy of the ACP registry has no such entry (`vessel agents catalogue --refresh` fetches it)",
        ))?;
    let CatalogueRun::Command {
        program,
        args,
        env,
        runner_found,
    } = &entry.runs
    else {
        return Err(CliError::Usage(
            "that registry entry runs no command here (`vessel agents catalogue` says what it is)",
        ));
    };
    let mut shown = format!(
        "{} {} would run on the node as:\n  {}",
        entry.id,
        entry.version,
        std::iter::once(program.as_str())
            .chain(args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    );
    for variable in env {
        let _infallible = write!(shown, "\n  with {}={}", variable.name, variable.value);
    }
    if !runner_found {
        let _infallible = write!(
            shown,
            "\n  ({program} is not on the node's search path yet)"
        );
    }
    io::note(&format!("{shown}\n")).await?;
    if !yes {
        io::note("Add it? [y/N] ").await?;
        let mut lines = io::input_lines()?;
        let answer = lines.recv().await.unwrap_or_default();
        if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            return Err(CliError::Usage("not added (you said no)"));
        }
    }
    Ok(client
        .call(AddAgent {
            name,
            agent: AgentToAdd::Registry {
                id: entry.id,
                version: entry.version,
                mcp,
            },
            replace,
        })
        .await?)
}

/// `vessel agents install`: show the command, ask, run it on the node and
/// print its output until it ends.
async fn install(
    client: &NodeClient,
    agent: HarnessKind,
    adapter: bool,
    yes: bool,
) -> Result<(), CliError> {
    let part = if adapter {
        InstallPart::Adapter
    } else {
        InstallPart::Harness
    };
    let listed = client.call(ListAgents {}).await?;
    let offer = listed
        .iter()
        .find_map(|info| match &info.detail {
            AgentDetail::Known { installs, .. } if info.agent == agent => {
                installs.iter().find(|offer| offer.part == part).cloned()
            }
            AgentDetail::Known { .. } | AgentDetail::Added { .. } => None,
        })
        .ok_or(CliError::Usage(
            "the node offers no such install (`vessel agents` lists each agent's)",
        ))?;
    let command = offer.command;
    if offer.by_hand {
        // A script or a download (ADR 0058): the node will not run it.
        io::note(&format!(
            "Vessel does not run this installer; on that machine, run:\n  {command}\n"
        ))
        .await?;
        return Ok(());
    }
    io::note(&format!(
        "The node would run, as its user, in its home:\n  {command}\n"
    ))
    .await?;
    if !yes {
        io::note("Run it? [y/N] ").await?;
        let mut lines = io::input_lines()?;
        let answer = lines.recv().await.unwrap_or_default();
        if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            return Err(CliError::Usage("not run (you said no)"));
        }
    }
    let job = client.call(InstallAgent { agent, part }).await?;
    let mut from = 0;
    loop {
        let page = client
            .call(ReadInstall {
                job: job.id,
                from,
                wait: Some(30),
            })
            .await?;
        from = page.next;
        io::out(&page.bytes.into_bytes()).await?;
        match page.state {
            InstallState::Running => {}
            InstallState::Succeeded => {
                return Ok(io::note(&format!("[vessel] {command}: done\n")).await?);
            }
            InstallState::Failed { code } => {
                let code = code.map_or_else(
                    || "no exit code".to_owned(),
                    |code| format!("exit code {code}"),
                );
                return Err(CliError::AgentInstall(format!(
                    "`{command}` failed ({code})"
                )));
            }
            InstallState::TimedOut => {
                return Err(CliError::AgentInstall(format!(
                    "`{command}` was stopped after 15 minutes"
                )));
            }
        }
    }
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
            from_registry: None,
            yes: false,
            command: vec!["goose".to_owned(), "acp".to_owned()],
        })
        .unwrap();
        let AgentToAdd::Command { definition } = &request.agent else {
            panic!("a command: {request:?}");
        };
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
