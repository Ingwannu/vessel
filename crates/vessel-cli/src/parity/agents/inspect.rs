//! Inspect projects known facts; unavailable capabilities and parentage remain unknown.
use super::{row, selected};
use crate::parity::{
    observe::{self, State},
    output::{self, Row},
};
use crate::{
    args::{Target, parity::Presentation},
    connect::connect,
    error::CliError,
    selector::SessionSelector,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use vessel_proto::{OptionCategory, PermissionId, SessionId, SessionMode, WorktreeId};

pub(super) async fn run(
    target: &Target,
    presentation: Presentation,
    id: SessionSelector,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let info = id.resolve(&client).await?;
    let observed = match info.spec.mode {
        SessionMode::Structured { .. } => Some(observe::snapshot(&client, info.id).await?),
        SessionMode::Terminal { .. } => None,
    };
    let options = observed
        .as_ref()
        .map_or(&[][..], |state| state.options.as_slice());
    let status = observed.as_ref().map_or_else(
        || {
            if info.status.is_live() {
                State::Running
            } else {
                State::Idle
            }
        },
        observe::Observed::state,
    );
    let inspection = Inspection {
        id: info.id,
        name: Text(
            info.title
                .as_ref()
                .map_or_else(|| "-".to_owned(), ToString::to_string),
        ),
        provider: Text(info.spec.harness.to_string()),
        model: Text(
            row::model(&info, options).map_or_else(|| "-".to_owned(), |model| model.to_string()),
        ),
        thinking: Text(
            selected(options, OptionCategory::ThoughtLevel)
                .map_or_else(|| "auto".to_owned(), ToString::to_string),
        ),
        status,
        archived: if info.marks.archived_at.is_some() {
            Archived::Yes
        } else {
            Archived::No
        },
        archived_at: info.marks.archived_at,
        mode: Text(
            selected(options, OptionCategory::Mode)
                .map_or_else(|| "default".to_owned(), ToString::to_string),
        ),
        cwd: Text(
            info.spec
                .cwd
                .as_ref()
                .map_or_else(|| "-".to_owned(), |path| path.display().to_string()),
        ),
        created_at: info.created_at,
        updated_at: observed
            .as_ref()
            .and_then(|state| state.modified_at)
            .unwrap_or(info.created_at),
        // These facts have no equivalent in the current node snapshot. In particular,
        // Creator authorization provenance must never masquerade as current parentage.
        last_usage: None,
        capabilities: None,
        available_modes: modes(options),
        pending_permissions: observed.as_ref().map_or_else(Vec::new, |state| {
            state
                .permission_ids()
                .map(|id| PendingPermission {
                    id,
                    tool: Text("unknown".to_owned()),
                })
                .collect()
        }),
        worktree: info
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.worktree),
        parent_agent_id: None,
    };
    match presentation {
        Presentation::Table(_) | Presentation::Quiet => {
            output::list(presentation, &inspection.rows()).await
        }
        Presentation::Json | Presentation::Yaml => {
            output::single(presentation, &info.id.to_string(), &inspection).await
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Inspection {
    id: SessionId,
    name: Text,
    provider: Text,
    model: Text,
    thinking: Text,
    status: State,
    archived: Archived,
    archived_at: Option<DateTime<Utc>>,
    mode: Text,
    cwd: Text,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_usage: Option<Unavailable>,
    capabilities: Option<Unavailable>,
    available_modes: Option<Vec<Mode>>,
    pending_permissions: Vec<PendingPermission>,
    worktree: Option<WorktreeId>,
    parent_agent_id: Option<SessionId>,
}
/// No success value exists for a fact the node has not supplied.
#[derive(Serialize)]
enum Unavailable {}
#[derive(Serialize)]
#[serde(transparent)]
struct Text(String);
enum Archived {
    Yes,
    No,
}
impl Serialize for Archived {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(matches!(self, Self::Yes))
    }
}
#[derive(Serialize)]
struct PendingPermission {
    id: PermissionId,
    tool: Text,
}
#[derive(Serialize)]
struct Mode {
    id: vessel_proto::OptionValueId,
    label: String,
}
fn modes(options: &[vessel_proto::SessionOption]) -> Option<Vec<Mode>> {
    options
        .iter()
        .find(|option| option.category == OptionCategory::Mode)
        .and_then(|option| match &option.kind {
            vessel_proto::OptionKind::Select { values, .. } => Some(
                values
                    .iter()
                    .map(|value| Mode {
                        id: value.id.clone(),
                        label: value.name.clone(),
                    })
                    .collect(),
            ),
            vessel_proto::OptionKind::Toggle { .. } | vessel_proto::OptionKind::Unknown => None,
        })
}
#[derive(Serialize)]
struct Field {
    key: &'static str,
    value: Text,
}
impl Row for Field {
    fn id(&self) -> String {
        self.key.to_owned()
    }
    fn headers() -> &'static [&'static str] {
        &["KEY", "VALUE"]
    }
    fn cells(&self) -> Vec<String> {
        vec![self.key.to_owned(), self.value.0.clone()]
    }
}
impl Inspection {
    fn rows(&self) -> Vec<Field> {
        let fields = [
            ("Id", self.id.to_string()),
            ("Name", self.name.0.clone()),
            ("Provider", self.provider.0.clone()),
            ("Model", self.model.0.clone()),
            ("Thinking", self.thinking.0.clone()),
            ("Status", self.status.label().to_owned()),
            (
                "Archived",
                matches!(self.archived, Archived::Yes).to_string(),
            ),
            (
                "ArchivedAt",
                self.archived_at
                    .map_or_else(|| "null".to_owned(), |at| at.to_rfc3339()),
            ),
            ("Mode", self.mode.0.clone()),
            ("Cwd", self.cwd.0.clone()),
            ("CreatedAt", self.created_at.to_rfc3339()),
            ("UpdatedAt", self.updated_at.to_rfc3339()),
            (
                "PendingPermissions",
                self.pending_permissions
                    .iter()
                    .map(|permission| format!("{} (unknown)", permission.id))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            (
                "Worktree",
                self.worktree
                    .map_or_else(|| "null".to_owned(), |id| id.to_string()),
            ),
            ("ParentAgentId", "null".to_owned()),
        ];
        let mut rows = fields
            .into_iter()
            .map(|(key, value)| Field {
                key,
                value: Text(value),
            })
            .collect::<Vec<_>>();
        if let Some(modes) = &self.available_modes
            && !modes.is_empty()
        {
            rows.push(Field {
                key: "AvailableModes",
                value: Text(
                    modes
                        .iter()
                        .map(|mode| format!("{} ({})", mode.id, mode.label))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            });
        }
        rows
    }
}
