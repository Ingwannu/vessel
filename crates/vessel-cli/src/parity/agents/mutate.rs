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
    let result = Stopped {
        stopped_count: Count(agent_ids.len()),
        agent_ids,
    };
    output::record(presentation, &result).await
}

pub(super) async fn delete(
    target: &Target,
    presentation: Presentation,
    selection: Selection,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let infos = select(&client, &selection).await?;
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
    output::record(presentation, &result).await
}

pub(super) async fn archive(
    target: &Target,
    presentation: Presentation,
    id: SessionSelector,
    intent: ArchiveIntent,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let info = id.resolve(&client).await?;
    if info.marks.archived_at.is_some() {
        return Err(CliError::Usage("agent is already archived"));
    }
    if intent == ArchiveIntent::IfIdle {
        if info.status.is_live() {
            structured(&info)?;
            if observe::snapshot(&client, info.id).await?.state() != State::Idle {
                return Err(CliError::Usage(
                    "agent has an active turn; --force explicitly interrupts before archiving",
                ));
            }
        }
        // Ended sessions can resume too: neither snapshot authorizes a force archive.
        // W21's admission must fence prompts and stored-session reopening atomically.
        return Err(CliError::ParityBlocked(
            "archive without --force requires the atomic archive-if-idle node call",
        ));
    }
    let archived = client.mark(info.id, Mark::Archive).await?;
    let result = ArchiveReceipt {
        agent_id: info.id,
        status: ArchivedStatus::Archived,
        archived_at: archived
            .marks
            .archived_at
            .ok_or(CliError::ParityInvalidReceipt(
                "archive acknowledgement omitted its archived timestamp",
            ))?,
    };
    output::record(presentation, &result).await
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ArchiveIntent {
    IfIdle,
    Force,
}
pub(super) enum ModeAction {
    List,
    Set(OptionValueId),
}

pub(super) async fn mode(
    target: &Target,
    presentation: Presentation,
    id: SessionSelector,
    action: ModeAction,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let info = id.resolve(&client).await?;
    structured(&info)?;
    let options = match &action {
        ModeAction::List => observe::snapshot(&client, info.id).await?.options,
        ModeAction::Set(_) => client.session_options(info.id).await?,
    };
    match action {
        ModeAction::List => {
            let rows = options
                .iter()
                .filter(|option| option.category == OptionCategory::Mode)
                .flat_map(|option| match &option.kind {
                    OptionKind::Select { values, .. } => values.as_slice(),
                    OptionKind::Toggle { .. } | OptionKind::Unknown => &[],
                })
                .map(|value| ModeRow {
                    id: value.id.clone(),
                    label: value.name.clone(),
                    description: value.description.clone(),
                })
                .collect::<Vec<_>>();
            output::list(presentation, &rows).await
        }
        ModeAction::Set(mode) => {
            let choice = choice(&options, OptionCategory::Mode, &mode)?;
            let option_id = choice.id.clone();
            let options = client.set_option(info.id, choice).await?;
            confirmed(&options, &option_id, &mode)?;
            let result = ModeSet {
                agent_id: super::row::ShortId::new(info.id),
                mode,
            };
            output::record(presentation, &result).await
        }
    }
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
        return Err(CliError::Usage(
            "update metadata and thinking in separate commands",
        ));
    }
    let client = connect(target).await?.client;
    let info = id.resolve(&client).await?;
    if let Some(thinking) = thinking {
        let options = client.session_options(info.id).await?;
        let choice = choice(&options, OptionCategory::ThoughtLevel, &thinking)?;
        let option_id = choice.id.clone();
        let options = client.set_option(info.id, choice).await?;
        confirmed(&options, &option_id, &thinking)?;
    }
    let info = if let Some(name) = name {
        client.rename(info.id, name).await?
    } else {
        client.session(info.id).await?
    };
    output::single(presentation, &info.id.to_string(), &info).await
}

/// Success describes the exact setting acknowledged, not merely a returned list.
fn confirmed(
    options: &[vessel_proto::SessionOption],
    id: &vessel_proto::OptionId,
    wanted: &OptionValueId,
) -> Result<(), CliError> {
    if options.iter().any(|option| {
        &option.id == id
            && matches!(&option.kind, OptionKind::Select { current, .. } if current == wanted)
    }) {
        return Ok(());
    }
    Err(CliError::ParityInvalidReceipt(
        "setting acknowledgement did not report the requested selection",
    ))
}

pub(super) fn choice(
    options: &[vessel_proto::SessionOption],
    category: OptionCategory,
    value: &OptionValueId,
) -> Result<OptionChoice, CliError> {
    let mut matching = options
        .iter()
        .filter(|option| option.category == category)
        .filter(|option| match &option.kind {
            OptionKind::Select { values, .. } => values.iter().any(|offered| &offered.id == value),
            OptionKind::Toggle { .. } | OptionKind::Unknown => false,
        });
    let option = matching.next().ok_or(CliError::Usage(
        "the requested value is not among the agent's selectable settings",
    ))?;
    if matching.next().is_some() {
        return Err(CliError::Usage(
            "the advertised value is ambiguous across settings; no option was changed",
        ));
    }
    Ok(OptionChoice {
        id: option.id.clone(),
        value: ChoiceValue::Select {
            value: value.clone(),
        },
    })
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
impl output::Row for Stopped {
    fn id(&self) -> String {
        self.agent_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn headers() -> &'static [&'static str] {
        &["INTERRUPTED"]
    }
    fn cells(&self) -> Vec<String> {
        vec![self.stopped_count.0.to_string()]
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Deleted {
    deleted_count: Count,
    agent_ids: Vec<vessel_proto::SessionId>,
}
impl output::Row for Deleted {
    fn id(&self) -> String {
        self.agent_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn headers() -> &'static [&'static str] {
        &["DELETED"]
    }
    fn cells(&self) -> Vec<String> {
        vec![self.deleted_count.0.to_string()]
    }
}
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ArchivedStatus {
    Archived,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveReceipt {
    agent_id: vessel_proto::SessionId,
    status: ArchivedStatus,
    archived_at: chrono::DateTime<chrono::Utc>,
}
impl output::Row for ArchiveReceipt {
    fn id(&self) -> String {
        self.agent_id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["AGENT ID", "STATUS", "ARCHIVED AT"]
    }
    fn cells(&self) -> Vec<String> {
        let status = match self.status {
            ArchivedStatus::Archived => "archived",
        };
        vec![
            self.agent_id.to_string(),
            status.to_owned(),
            self.archived_at.to_rfc3339(),
        ]
    }
}

#[derive(serde::Serialize)]
struct ModeRow {
    id: OptionValueId,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}
impl output::Row for ModeRow {
    fn id(&self) -> String {
        self.id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["MODE", "LABEL", "DESCRIPTION"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.label.clone(),
            self.description.clone().unwrap_or_default(),
        ]
    }
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ModeSet {
    agent_id: super::row::ShortId,
    mode: OptionValueId,
}
impl output::Row for ModeSet {
    fn id(&self) -> String {
        self.agent_id.as_str().to_owned()
    }
    fn headers() -> &'static [&'static str] {
        &["AGENT ID", "MODE"]
    }
    fn cells(&self) -> Vec<String> {
        vec![self.agent_id.as_str().to_owned(), self.mode.to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn offered(id: &str, value: &str) -> vessel_proto::SessionOption {
        vessel_proto::SessionOption {
            id: vessel_proto::OptionId::new(id),
            name: id.to_owned(),
            description: None,
            category: OptionCategory::Mode,
            kind: OptionKind::Select {
                current: OptionValueId::new(value),
                values: vec![vessel_proto::OptionValue {
                    id: OptionValueId::new(value),
                    name: value.to_owned(),
                    description: None,
                    group: None,
                }],
            },
        }
    }
    #[test]
    fn value_resolution_uses_the_actual_option_identity_and_refuses_ambiguity() {
        let options = [offered("first", "read"), offered("second", "plan")];
        let selected = choice(&options, OptionCategory::Mode, &OptionValueId::new("plan")).unwrap();
        assert_eq!(selected.id, vessel_proto::OptionId::new("second"));
        assert!(
            choice(
                &[offered("first", "same"), offered("second", "same")],
                OptionCategory::Mode,
                &OptionValueId::new("same")
            )
            .is_err()
        );
        assert!(
            choice(
                &options,
                OptionCategory::Mode,
                &OptionValueId::new("unknown")
            )
            .is_err()
        );
    }
    #[test]
    fn setting_success_requires_the_same_option_and_requested_value() {
        let options = [offered("actual", "new")];
        assert!(
            confirmed(
                &options,
                &vessel_proto::OptionId::new("actual"),
                &OptionValueId::new("new")
            )
            .is_ok()
        );
        assert!(
            confirmed(
                &options,
                &vessel_proto::OptionId::new("other"),
                &OptionValueId::new("new")
            )
            .is_err()
        );
        assert!(
            confirmed(
                &options,
                &vessel_proto::OptionId::new("actual"),
                &OptionValueId::new("old")
            )
            .is_err()
        );
    }
}
