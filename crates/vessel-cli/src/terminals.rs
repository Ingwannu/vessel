//! `vessel shell`, `vessel script` and `vessel ports` (protocol 11, ADR
//! 0031). A terminal is named by its id or a unique prefix of one; a
//! session as everywhere else; a project by id, prefix or name.

use std::fmt::Write as _;

use vessel_client::NodeClient;
use vessel_proto::EventSeq;
use vessel_proto::ports::{ListPorts, PortList, PortOwner, PortScan};
use vessel_proto::scripts::{
    ListScripts, RestartScript, RunScript, ScriptPlace, ScriptRefusal, ScriptRunOutcome,
};
use vessel_proto::terminals::{
    CloseTerminal, CreateTerminal, KillTerminal, ListTerminals, RenameTerminal, TerminalInfo,
    TerminalKind, TerminalPlace,
};

use crate::args::{OpenShell, Ports, ScriptCommand, ScriptPlaceArgs, Shell, ShellCommand, Target};
use crate::attach;
use crate::connect::{Connected, connect};
use crate::error::CliError;
use crate::render;
use crate::workspace::{commands_text, find_project, one, out, parse};

/// `vessel shell …`.
pub(crate) async fn shell(target: &Target, shell: Shell) -> Result<(), CliError> {
    let connected = connect(target).await?;
    let client = &connected.client;
    let Some(command) = shell.command else {
        return open(&connected, shell.open).await;
    };
    match command {
        ShellCommand::List { session, project } => {
            let place = match (session, project) {
                (Some(session), _) => Some(TerminalPlace::Session {
                    session: session.resolve(client).await?.id,
                }),
                (None, Some(project)) => Some(TerminalPlace::Project {
                    project: find_project(client, &project).await?.id,
                    dir: None,
                }),
                (None, None) => None,
            };
            let list = client.call(ListTerminals { place }).await?;
            let mut text = String::new();
            for terminal in &list.terminals {
                text.push_str(&terminal_line(terminal));
            }
            out(&text).await
        }
        ShellCommand::Attach { terminal, from } => {
            let terminal = find_terminal(client, &terminal).await?;
            attach::attach_terminal(&connected, &terminal, from).await
        }
        ShellCommand::Rename { terminal, title } => {
            let terminal = find_terminal(client, &terminal).await?;
            let renamed = client
                .call(RenameTerminal {
                    terminal: terminal.id,
                    title: parse(&title, "title")?,
                })
                .await?;
            out(&terminal_line(&renamed)).await
        }
        ShellCommand::Kill { terminal } => {
            let terminal = find_terminal(client, &terminal).await?;
            let _ack = client
                .call(KillTerminal {
                    terminal: terminal.id,
                })
                .await?;
            out(&format!("killed {} (kept with its output)\n", terminal.id)).await
        }
        ShellCommand::Close { terminal } => {
            let terminal = find_terminal(client, &terminal).await?;
            let _ack = client
                .call(CloseTerminal {
                    terminal: terminal.id,
                })
                .await?;
            out(&format!("closed {}\n", terminal.id)).await
        }
    }
}

/// `vessel shell [--session | --project [--dir]]`: open one, and attach
/// unless `--detached`.
async fn open(connected: &Connected, open: OpenShell) -> Result<(), CliError> {
    let client = &connected.client;
    let place = match (open.session, open.project) {
        (Some(session), _) => TerminalPlace::Session {
            session: session.resolve(client).await?.id,
        },
        (None, Some(project)) => TerminalPlace::Project {
            project: find_project(client, &project).await?.id,
            dir: open.dir.map(|dir| parse(&dir, "dir")).transpose()?,
        },
        (None, None) => {
            return Err(CliError::Workspace(
                "give `--session` or `--project`: where the shell runs".to_owned(),
            ));
        }
    };
    let title = open.title.map(|title| parse(&title, "title")).transpose()?;
    let terminal = client
        .call(CreateTerminal {
            place,
            size: attach::terminal::window_size(),
            title,
        })
        .await?;
    if open.detached {
        return out(&terminal_line(&terminal)).await;
    }
    attach::attach_terminal(connected, &terminal, EventSeq::ZERO).await
}

/// `vessel script …`.
pub(crate) async fn script(target: &Target, command: ScriptCommand) -> Result<(), CliError> {
    let connected = connect(target).await?;
    let client = &connected.client;
    match command {
        ScriptCommand::List { place } => {
            let place = script_place(client, place).await?;
            let list = client.call(ListScripts { place }).await?;
            let mut text = format!("{}\n", list.root.display());
            text.push_str(&commands_text(&list.commands));
            for run in &list.runs {
                text.push_str(&terminal_line(run));
            }
            out(&text).await
        }
        ScriptCommand::Run {
            script,
            place,
            detached,
        } => {
            let place = script_place(client, place).await?;
            let outcome = client
                .call(RunScript {
                    place,
                    script: parse(&script, "script")?,
                })
                .await?;
            let run = started(outcome)?;
            out(&terminal_line(&run)).await?;
            if detached {
                return Ok(());
            }
            attach::attach_terminal(&connected, &run, EventSeq::ZERO).await
        }
        ScriptCommand::Restart { terminal } => {
            let terminal = find_terminal(client, &terminal).await?;
            let outcome = client
                .call(RestartScript {
                    terminal: terminal.id,
                })
                .await?;
            out(&terminal_line(&started(outcome)?)).await
        }
        ScriptCommand::Stop { terminal } => {
            let terminal = find_terminal(client, &terminal).await?;
            let _ack = client
                .call(KillTerminal {
                    terminal: terminal.id,
                })
                .await?;
            out(&format!("stopped {}\n", terminal.id)).await
        }
        ScriptCommand::Output { terminal, from } => {
            let terminal = find_terminal(client, &terminal).await?;
            attach::attach_terminal(&connected, &terminal, from).await
        }
    }
}

/// `vessel ports [--watch]`.
pub(crate) async fn ports(target: &Target, ports: &Ports) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let mut list = client.call(ListPorts { changed_from: None }).await?;
    // The first answer may come before the first scan: wait for it.
    while matches!(list.scan, PortScan::Pending) {
        list = client
            .call(ListPorts {
                changed_from: Some(list.revision),
            })
            .await?;
    }
    out(&ports_text(&list)).await?;
    if !ports.watch {
        return Ok(());
    }
    // The receiver must outlive each printed snapshot, including a slow pipe.
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);
    loop {
        let next = tokio::select! {
            biased;
            interrupted = &mut interrupt => {
                interrupted?;
                return Ok(());
            }
            next = client.call(ListPorts { changed_from: Some(list.revision) }) => next?,
        };
        if next.revision != list.revision {
            out(&format!("\n{}", ports_text(&next))).await?;
        }
        list = next;
    }
}

/// The terminal `given` names: an id or a unique prefix of one.
async fn find_terminal(client: &NodeClient, given: &str) -> Result<TerminalInfo, CliError> {
    let terminals = client.call(ListTerminals { place: None }).await?.terminals;
    let matching: Vec<TerminalInfo> = terminals
        .into_iter()
        .filter(|terminal| terminal.id.to_string().starts_with(given))
        .collect();
    one(matching, given, "terminal")
}

/// `--session` or `--project`: the checkout a script runs in.
async fn script_place(
    client: &NodeClient,
    place: ScriptPlaceArgs,
) -> Result<ScriptPlace, CliError> {
    match (place.session, place.project) {
        (Some(session), _) => Ok(ScriptPlace::Session {
            session: session.resolve(client).await?.id,
        }),
        (None, Some(project)) => Ok(ScriptPlace::Project {
            project: find_project(client, &project).await?.id,
        }),
        (None, None) => Err(CliError::Workspace(
            "give `--session` or `--project`: the checkout whose scripts to run".to_owned(),
        )),
    }
}

/// The run a `script.run` or `script.restart` started, or why none.
fn started(outcome: ScriptRunOutcome) -> Result<TerminalInfo, CliError> {
    match outcome {
        ScriptRunOutcome::Started { terminal } => Ok(*terminal),
        ScriptRunOutcome::NeedsApproval { commands } => Err(CliError::Workspace(format!(
            "the project's commands are not approved; check them and run \
             `vessel project approve <project> <digest>`:\n{}",
            commands_text(&commands)
        ))),
        ScriptRunOutcome::Refused { reason } => Err(CliError::Workspace(refusal_text(&reason))),
    }
}

fn refusal_text(reason: &ScriptRefusal) -> String {
    match reason {
        ScriptRefusal::NoProject => "the session runs in no project".to_owned(),
        ScriptRefusal::NoSuchScript => "vessel.toml names no such script or service".to_owned(),
        ScriptRefusal::NoSuchDir => "the script's `dir` is not a folder in the checkout".to_owned(),
        ScriptRefusal::InvalidFile { message } => format!("vessel.toml cannot be used: {message}"),
        ScriptRefusal::AlreadyRunning { terminal } => {
            format!("it runs already, in terminal {terminal}")
        }
        ScriptRefusal::Unknown => "the node refused it".to_owned(),
    }
}

/// One terminal as a line: id, kind, state, place, title.
fn terminal_line(terminal: &TerminalInfo) -> String {
    let kind = match &terminal.kind {
        TerminalKind::Shell { program } => format!("shell {}", program.display()),
        TerminalKind::Profile { profile, .. } => format!("profile {profile}"),
        TerminalKind::Script { script, .. } => format!("script {script}"),
        TerminalKind::Service { script, .. } => format!("service {script}"),
        TerminalKind::Unknown => "terminal".to_owned(),
    };
    let state = match (&terminal.service, &terminal.status) {
        (Some(service), _) => service.summary(),
        (None, status) if status.is_live() => "running".to_owned(),
        (None, status) => render::status(status),
    };
    let place = match &terminal.place {
        TerminalPlace::Session { session } => format!("beside {session}"),
        TerminalPlace::Project { project, .. } => format!("in project {project}"),
        TerminalPlace::Workspace { workspace } => format!("in workspace {workspace}"),
    };
    let title = terminal
        .title
        .as_ref()
        .map_or_else(String::new, |title| format!("  \"{title}\""));
    format!("{}  {kind}  {state}  {place}{title}\n", terminal.id)
}

fn ports_text(list: &PortList) -> String {
    let mut text = String::new();
    match &list.scan {
        PortScan::Pending => text.push_str("(not scanned yet)\n"),
        PortScan::Scanned { .. } => {}
        PortScan::Unavailable { reason } => {
            let _written = writeln!(text, "ports cannot be found here: {reason}");
        }
        PortScan::Unknown => text.push_str("(the node's scan says something newer)\n"),
    }
    for port in &list.ports {
        let owner = match &port.owner {
            PortOwner::Session { session } => format!("session {session}"),
            PortOwner::Terminal { terminal } => format!("terminal {terminal}"),
        };
        let _written = writeln!(
            text,
            "{}:{}  {}  pid {}  {owner}",
            port.address,
            port.port,
            port.process.as_ref().map_or("-", |name| name.as_str()),
            port.pid,
        );
    }
    if list.more {
        text.push_str("(more not shown)\n");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_says_what_to_do() {
        assert!(refusal_text(&ScriptRefusal::NoSuchScript).contains("no such script"));
        let terminal = vessel_proto::TerminalId::random();
        assert!(
            refusal_text(&ScriptRefusal::AlreadyRunning { terminal })
                .contains(&terminal.to_string())
        );
    }
}
