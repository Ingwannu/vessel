//! Agent list, inspection and lifecycle shortcuts reuse the typed node client.
//! Provider creation and rich permissions keep their owning packages' contracts.

mod detach;
mod inspect;
mod mutate;
mod prompt;
mod receipt;
mod row;
use crate::labels::parse as labels;
use row::AgentRow;

use super::{
    observe::{self, State},
    output,
};
use crate::{
    args::{
        Target,
        parity::{AgentCommand, AgentList, Presentation, Selection},
    },
    connect::connect,
    error::CliError,
};
use std::path::{Path, PathBuf};
use vessel_client::NodeClient;
use vessel_proto::{SessionInfo, SessionMode, SessionOption};

pub(super) async fn run(
    target: &Target,
    presentation: Presentation,
    command: AgentCommand,
) -> Result<(), CliError> {
    match command {
        AgentCommand::Ls(args) => list(target, presentation, args).await,
        AgentCommand::Run(args) => prompt::run(target, presentation, *args).await,
        AgentCommand::Send(args) => prompt::send(target, presentation, args).await,
        AgentCommand::Import {
            id,
            provider,
            cwd,
            label,
        } => prompt::import(target, presentation, id, provider, cwd, label).await,
        AgentCommand::Inspect { id } => inspect::run(target, presentation, id).await,
        AgentCommand::Wait(args) => {
            let client = connect(target).await?.client;
            let info = args.id.resolve(&client).await?;
            structured(&info)?;
            let result = observe::wait(&client, info.id, args.timeout).await?;
            receipt::wait_finished(presentation, &result).await
        }
        AgentCommand::Stop(selection) => mutate::stop(target, presentation, selection).await,
        AgentCommand::Delete(selection) => mutate::delete(target, presentation, selection).await,
        AgentCommand::Archive { id, force } => {
            let intent = if force {
                mutate::ArchiveIntent::Force
            } else {
                mutate::ArchiveIntent::IfIdle
            };
            mutate::archive(target, presentation, id, intent).await
        }
        AgentCommand::Mode { id, mode, list } => {
            let action = if list {
                mutate::ModeAction::List
            } else {
                mutate::ModeAction::Set(mode.ok_or(CliError::Usage("give a mode or --list"))?)
            };
            mutate::mode(target, presentation, id, action).await
        }
        AgentCommand::Update { id, name, thinking } => {
            mutate::update(target, presentation, id, name, thinking).await
        }
        AgentCommand::Attach { id } => {
            let connected = connect(target).await?;
            let info = id.resolve(&connected.client).await?;
            crate::attach::attach(&connected, &info, vessel_proto::EventSeq::ZERO).await
        }
        AgentCommand::Detach { id } => detach::run(target, presentation, id).await,
        AgentCommand::Open { .. } => Err(CliError::ParityBlocked(
            "agent open awaits the desktop deep-link contract",
        )),
    }
}

pub(super) fn structured(info: &SessionInfo) -> Result<(), CliError> {
    match info.spec.mode {
        SessionMode::Structured { .. } => Ok(()),
        SessionMode::Terminal { .. } => Err(CliError::Usage(
            "this operation requires a structured agent; terminal process state is not turn state",
        )),
    }
}

/// Scope before mutation; --all is explicit authority, not an ID fallback.
pub(super) async fn select(
    client: &NodeClient,
    selection: &Selection,
) -> Result<Vec<SessionInfo>, CliError> {
    if let Some(id) = &selection.id {
        return Ok(vec![id.resolve(client).await?]);
    }
    let infos = client.sessions().await?;
    let cwd = selection
        .cwd
        .as_ref()
        .map(|path| absolute(path))
        .transpose()?;
    Ok(infos
        .into_iter()
        .filter(|info| {
            selection.all
                || cwd.as_ref().is_some_and(|path| {
                    info.spec
                        .cwd
                        .as_ref()
                        .is_some_and(|cwd| cwd.starts_with(path))
                })
        })
        .collect())
}
fn absolute(path: &Path) -> Result<PathBuf, CliError> {
    Ok(std::path::absolute(path)?)
}

async fn list(
    target: &Target,
    presentation: Presentation,
    args: AgentList,
) -> Result<(), CliError> {
    let filters = labels(&args.label)?;
    if args.background {
        return Err(CliError::ParityBlocked(
            "--background awaits workspace background visibility on the node",
        ));
    }
    let cwd = if args.all || args.global {
        None
    } else {
        Some(std::env::current_dir()?)
    };
    let client = connect(target).await?.client;
    let sessions = if filters.is_empty() {
        client.sessions().await?
    } else {
        // Legacy projection can erase labels. Explicit filtering must pass the
        // producer's metadata gate instead of claiming an empty local result.
        client
            .call(vessel_proto::ListSessions::selected(
                vessel_proto::SessionSelection::all().with_labels(filters.clone()),
            ))
            .await?
    };
    let mut rows = Vec::new();
    for info in sessions {
        if !matches_scope(&info, cwd.as_deref(), args.all, &filters) {
            continue;
        }
        // Option snapshots survive process exit in the event log; session.options
        // deliberately refuses ended agents. One replay supplies both projections.
        let observed = match info.spec.mode {
            SessionMode::Structured { .. } => Some(observe::snapshot(&client, info.id).await?),
            SessionMode::Terminal { .. } => None,
        };
        let thinking = observed.as_ref().and_then(|observed| {
            selected(
                &observed.options,
                vessel_proto::OptionCategory::ThoughtLevel,
            )
        });
        if args
            .thinking
            .as_ref()
            .is_some_and(|wanted| thinking != Some(wanted))
        {
            continue;
        }
        let state = observed.as_ref().map_or_else(
            || {
                if info.status.is_live() {
                    State::Running
                } else {
                    State::Idle
                }
            },
            observe::Observed::state,
        );
        let options = observed
            .as_ref()
            .map_or(&[][..], |observed| observed.options.as_slice());
        rows.push(AgentRow::new(&info, state, thinking, options));
    }
    output::list(presentation, &rows).await
}

fn matches_scope(
    info: &SessionInfo,
    cwd: Option<&Path>,
    all: bool,
    filters: &vessel_proto::SessionLabels,
) -> bool {
    (all || info.marks.archived_at.is_none())
        && cwd.is_none_or(|cwd| info.spec.cwd.as_deref() == Some(cwd))
        && info.labels.covers(filters)
}

pub(super) fn selected(
    options: &[SessionOption],
    category: vessel_proto::OptionCategory,
) -> Option<&vessel_proto::OptionValueId> {
    options
        .iter()
        .find(|option| option.category == category)
        .and_then(|option| match &option.kind {
            vessel_proto::OptionKind::Select { current, .. } => Some(current),
            vessel_proto::OptionKind::Toggle { .. } | vessel_proto::OptionKind::Unknown => None,
        })
}
