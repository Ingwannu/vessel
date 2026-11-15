//! Mutating shortcuts validate the full selection/options before their first effect.

use super::{select, structured};
use crate::parity::{
    observe::{self, State},
    output,
};
use crate::{
    args::{
        Target,
        parity::{Presentation, Selection},
    },
    connect::connect,
    error::CliError,
    selector::SessionSelector,
};
use vessel_proto::{
    ChoiceValue, Mark, OptionCategory, OptionChoice, OptionKind, OptionValueId, SessionInput,
    SessionTitle,
};

pub(super) async fn stop(
    target: &Target,
    presentation: Presentation,
    selection: Selection,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let infos = select(&client, &selection)
        .await?
        .into_iter()
        .filter(|info| info.marks.archived_at.is_none())
        .collect::<Vec<_>>();
    // Resolve every input before mutating any selected session.
    for info in &infos {
        structured(info)?;
    }
    let mut turns = Vec::new();
    for info in &infos {
        turns.push((info.id, observe::snapshot(&client, info.id).await?.turn));
    }
    let mut agent_ids = Vec::new();
    for (id, turn) in turns {
        if let Some(turn) = turn {
            client
                .send(id, SessionInput::CancelTurn { turn: Some(turn) })
                .await?;
            agent_ids.push(id);
        }
    }
    let ids = agent_ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let result = Stopped {
        stopped_count: Count(agent_ids.len()),
        agent_ids,
    };
    output::single(presentation, &ids, &result).await
}

pub(super) async fn delete(
    target: &Target,
    presentation: Presentation,
    selection: Selection,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let infos = select(&client, &selection).await?;
    let ids = infos
        .iter()
        .map(|info| info.id.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut deleted = Vec::new();
    for info in infos {
        // mark archive waits for process termination; unlike kill's ack,
        // it guarantees session.delete will not race an active actor.
        if info.status.is_live() {
            let _archived = client.mark(info.id, Mark::Archive).await?;
        }
        let _receipt = client.delete_session(info.id).await?;
        deleted.push(info.id);
    }
    let result = Deleted {
        deleted_count: Count(deleted.len()),
        agent_ids: deleted,
    };
    output::single(presentation, &ids, &result).await
}

pub(super) async fn archive(
    target: &Target,
    presentation: Presentation,
    id: SessionSelector,
    force: bool,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let info = id.resolve(&client).await?;
    if info.marks.archived_at.is_some() {
        return Err(CliError::Usage("agent is already archived"));
    }
    if !force && info.status.is_live() {
        structured(&info)?;
        if observe::snapshot(&client, info.id).await?.state() != State::Idle {
            return Err(CliError::Usage(
                "agent has an active turn; --force explicitly interrupts before archiving",
            ));
        }
        // This preflight can race another caller's prompt. An atomic node
        // archive-if-idle contract is required before exposing this case.
        return Err(CliError::ParityBlocked(
            "archiving a live idle agent without --force requires an atomic archive-if-idle node call",
        ));
    }
    let archived = client.mark(info.id, Mark::Archive).await?;
    let result = Archived {
        agent_id: info.id,
        status: ArchivedStatus::Archived,
        archived_at: archived
            .marks
            .archived_at
            .ok_or(CliError::ParityInvalidReceipt(
                "archive acknowledgement omitted its archived timestamp",
            ))?,
    };
    output::single(presentation, &info.id.to_string(), &result).await
}

pub(super) async fn mode(
    target: &Target,
    presentation: Presentation,
    id: SessionSelector,
    mode: Option<OptionValueId>,
    list: bool,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let info = id.resolve(&client).await?;
    let options = client.session_options(info.id).await?;
    if list || mode.is_none() {
        return output::single(presentation, &info.id.to_string(), &options).await;
    }
    let choice = choice(
        &options,
        OptionCategory::Mode,
        mode.as_ref().ok_or(CliError::Usage("give a mode"))?,
    )?;
    let options = client.set_option(info.id, choice).await?;
    output::single(presentation, &info.id.to_string(), &options).await
}

pub(super) async fn update(
    target: &Target,
    presentation: Presentation,
    id: SessionSelector,
    name: Option<SessionTitle>,
    thinking: Option<OptionValueId>,
) -> Result<(), CliError> {
    if name.is_none() && thinking.is_none() {
        return Err(CliError::Usage("give --name or --thinking"));
    }
    if name.is_some() && thinking.is_some() {
        return Err(CliError::ParityBlocked(
            "combined name/thinking update needs the atomic provider configuration contract",
        ));
    }
    let client = connect(target).await?.client;
    let info = id.resolve(&client).await?;
    if let Some(thinking) = thinking {
        let options = client.session_options(info.id).await?;
        let choice = choice(&options, OptionCategory::ThoughtLevel, &thinking)?;
        let _options = client.set_option(info.id, choice).await?;
    }
    let info = if let Some(name) = name {
        client.rename(info.id, name).await?
    } else {
        client.session(info.id).await?
    };
    output::single(presentation, &info.id.to_string(), &info).await
}

pub(super) fn choice(
    options: &[vessel_proto::SessionOption],
    category: OptionCategory,
    value: &OptionValueId,
) -> Result<OptionChoice, CliError> {
    let option = options
        .iter()
        .find(|option| option.category == category)
        .ok_or(CliError::Usage("the agent does not advertise this setting"))?;
    match &option.kind {
        OptionKind::Select { values, .. } if values.iter().any(|offered| &offered.id == value) => {
            Ok(OptionChoice {
                id: option.id.clone(),
                value: ChoiceValue::Select {
                    value: value.clone(),
                },
            })
        }
        OptionKind::Select { .. } => Err(CliError::Usage(
            "the requested value is not among the agent's offered values",
        )),
        OptionKind::Toggle { .. } | OptionKind::Unknown => Err(CliError::Usage(
            "the agent does not advertise a selectable setting",
        )),
    }
}

#[derive(serde::Serialize)]
#[serde(transparent)]
struct Count(usize);
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Stopped {
    stopped_count: Count,
    agent_ids: Vec<vessel_proto::SessionId>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Deleted {
    deleted_count: Count,
    agent_ids: Vec<vessel_proto::SessionId>,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ArchivedStatus {
    Archived,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Archived {
    agent_id: vessel_proto::SessionId,
    status: ArchivedStatus,
    archived_at: chrono::DateTime<chrono::Utc>,
}
