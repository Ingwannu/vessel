//! Schedule list projections share one typed source for human and structured output.
use super::output::{self, Row};
use crate::{args::parity::Presentation, error::CliError};
use chrono::{DateTime, Utc};
use serde::Serialize;
use vessel_proto::{
    ScheduleId, SessionMode,
    automation::{Cadence, ScheduleInfo, ScheduleState, ScheduleTarget},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScheduleRow {
    id: ScheduleId,
    name: Option<vessel_proto::automation::ScheduleName>,
    cadence: CadenceText,
    target: TargetText,
    status: Status,
    next_run_at: Option<DateTime<Utc>>,
    last_run_at: Option<DateTime<Utc>>,
}
#[derive(Serialize)]
#[serde(transparent)]
struct CadenceText(String);
#[derive(Serialize)]
#[serde(transparent)]
struct TargetText(String);
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Active,
    Paused,
    Completed,
    Unknown,
}
impl From<&ScheduleInfo> for ScheduleRow {
    fn from(info: &ScheduleInfo) -> Self {
        let cadence = match &info.definition.cadence {
            Cadence::Cron { expr, zone } => format!("cron:{} ({})", expr.as_str(), zone.as_str()),
            Cadence::Every { seconds, .. } => format!("every:{}s", seconds.get()),
            Cadence::Unknown => "unknown".to_owned(),
        };
        let target = match &info.definition.target {
            ScheduleTarget::NewSession { spec, .. } => {
                let model = match &spec.mode {
                    SessionMode::Structured {
                        model: Some(model), ..
                    } => format!("/{model}"),
                    SessionMode::Structured { model: None, .. } | SessionMode::Terminal { .. } => {
                        String::new()
                    }
                };
                format!("new-agent:{}{model}", spec.harness)
            }
            ScheduleTarget::Session { session } => format!(
                "agent:{}",
                session.to_string().chars().take(7).collect::<String>()
            ),
            ScheduleTarget::Unknown => "unknown".to_owned(),
        };
        let (status, next_run_at) = match info.state {
            ScheduleState::Active { next_due } => (Status::Active, Some(next_due)),
            ScheduleState::Paused { .. } => (Status::Paused, None),
            ScheduleState::Ended { .. } => (Status::Completed, None),
            ScheduleState::Unknown => (Status::Unknown, None),
        };
        Self {
            id: info.id,
            name: info.definition.name.clone(),
            cadence: CadenceText(cadence),
            target: TargetText(target),
            status,
            next_run_at,
            last_run_at: info.last_run.as_ref().map(|run| run.at),
        }
    }
}
impl Row for ScheduleRow {
    fn id(&self) -> String {
        self.id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["ID", "NAME", "CADENCE", "TARGET", "STATUS", "NEXT RUN"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.name
                .as_ref()
                .map_or_else(String::new, |name| name.as_str().to_owned()),
            self.cadence.0.clone(),
            self.target.0.clone(),
            match self.status {
                Status::Active => "active",
                Status::Paused => "paused",
                Status::Completed => "completed",
                Status::Unknown => "unknown",
            }
            .to_owned(),
            self.next_run_at
                .map_or_else(String::new, |at| at.to_rfc3339()),
        ]
    }
}
pub(crate) async fn one(presentation: Presentation, info: &ScheduleInfo) -> Result<(), CliError> {
    let row = ScheduleRow::from(info);
    output::record(presentation, &row).await
}
pub(crate) async fn inspect(
    presentation: Presentation,
    info: &ScheduleInfo,
    details: &str,
) -> Result<(), CliError> {
    match presentation {
        Presentation::Table(_) => Ok(crate::io::out(details.as_bytes()).await?),
        Presentation::Quiet | Presentation::Json | Presentation::Yaml => {
            output::single(presentation, &info.id.to_string(), info).await
        }
    }
}
pub(crate) async fn deleted(presentation: Presentation, id: ScheduleId) -> Result<(), CliError> {
    #[derive(Serialize)]
    struct Deleted {
        id: ScheduleId,
        deleted: output::Success,
    }
    output::single(
        presentation,
        &id.to_string(),
        &Deleted {
            id,
            deleted: output::Success,
        },
    )
    .await
}
