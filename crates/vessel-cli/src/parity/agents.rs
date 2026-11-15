//! Agent list, inspection and lifecycle shortcuts reuse the typed node client.
//! Provider creation and rich permissions keep their owning packages' contracts.

mod mutate;
mod prompt;

use super::{
    observe::{self, State},
    output::{self, Row},
};
use crate::{
    args::{
        Target,
        parity::{AgentCommand, AgentList, Presentation, Selection},
    },
    connect::connect,
    error::CliError,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use vessel_client::NodeClient;
use vessel_proto::{SessionId, SessionInfo, SessionMode, SessionOption};

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
        AgentCommand::Inspect { id } => {
            let client = connect(target).await?.client;
            let info = id.resolve(&client).await?;
            output::single(presentation, &info.id.to_string(), &info).await
        }
        AgentCommand::Wait(args) => {
            let client = connect(target).await?.client;
            let info = args.id.resolve(&client).await?;
            structured(&info)?;
            let result = observe::wait(&client, info.id, args.timeout).await?;
            output::single(presentation, &info.id.to_string(), &result).await
        }
        AgentCommand::Stop(selection) => mutate::stop(target, presentation, selection).await,
        AgentCommand::Delete(selection) => mutate::delete(target, presentation, selection).await,
        AgentCommand::Archive { id, force } => {
            mutate::archive(target, presentation, id, force).await
        }
        AgentCommand::Mode { id, mode, list } => {
            mutate::mode(target, presentation, id, mode, list).await
        }
        AgentCommand::Update { id, name, thinking } => {
            mutate::update(target, presentation, id, name, thinking).await
        }
        AgentCommand::Attach { id } => {
            let connected = connect(target).await?;
            let info = id.resolve(&connected.client).await?;
            crate::attach::attach(&connected, &info, vessel_proto::EventSeq::ZERO).await
        }
        AgentCommand::Detach { .. } => Err(CliError::ParityBlocked(
            "agent detach awaits the reviewed subagent attachment contract",
        )),
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

pub(super) fn labels(raw: &[String]) -> Result<vessel_proto::SessionLabels, CliError> {
    let mut labels = vessel_proto::SessionLabels::new();
    for text in raw {
        let (key, value) = text
            .split_once('=')
            .ok_or(CliError::Usage("labels must be key=value"))?;
        labels
            .insert(
                key.parse().map_err(CliError::ParityLabel)?,
                value.parse().map_err(CliError::ParityLabel)?,
            )
            .map_err(CliError::ParityLabel)?;
    }
    Ok(labels)
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
                || cwd
                    .as_ref()
                    .is_some_and(|path| info.spec.cwd.as_ref() == Some(path))
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
    let mut rows = Vec::new();
    for info in client.sessions().await? {
        if !matches_scope(&info, cwd.as_deref(), args.all, &filters) {
            continue;
        }
        let options = match info.spec.mode {
            SessionMode::Structured { .. } => client.session_options(info.id).await?,
            SessionMode::Terminal { .. } => Vec::new(),
        };
        let thinking = selected(&options, vessel_proto::OptionCategory::ThoughtLevel);
        if args
            .thinking
            .as_ref()
            .is_some_and(|wanted| thinking != Some(wanted))
        {
            continue;
        }
        let state = match info.spec.mode {
            SessionMode::Structured { .. } => observe::snapshot(&client, info.id).await?.state(),
            SessionMode::Terminal { .. } => {
                if info.status.is_live() {
                    State::Running
                } else {
                    State::Idle
                }
            }
        };
        rows.push(AgentRow::new(&info, state, thinking));
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentRow {
    id: SessionId,
    short_id: String,
    name: String,
    provider: String,
    thinking: String,
    status: State,
    cwd: Option<PathBuf>,
    created: String,
}
impl AgentRow {
    fn new(
        info: &SessionInfo,
        status: State,
        thinking: Option<&vessel_proto::OptionValueId>,
    ) -> Self {
        let id = info.id.to_string();
        Self {
            id: info.id,
            short_id: id.chars().take(7).collect(),
            name: info
                .title
                .as_ref()
                .map_or_else(|| "-".to_owned(), ToString::to_string),
            provider: info.spec.harness.to_string(),
            thinking: thinking.map_or_else(|| "auto".to_owned(), ToString::to_string),
            status,
            cwd: info.spec.cwd.clone(),
            created: info.created_at.to_rfc3339(),
        }
    }
}
impl Row for AgentRow {
    fn id(&self) -> String {
        self.short_id.clone()
    }
    fn headers() -> &'static [&'static str] {
        &[
            "AGENT ID", "NAME", "PROVIDER", "THINKING", "STATUS", "CWD", "CREATED",
        ]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.short_id.clone(),
            self.name.clone(),
            self.provider.clone(),
            self.thinking.clone(),
            format!("{:?}", self.status).to_ascii_lowercase(),
            self.cwd
                .as_ref()
                .map_or_else(|| "-".to_owned(), |p| p.display().to_string()),
            self.created.clone(),
        ]
    }
}
