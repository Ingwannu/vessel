//! Display projections retain the node's offered model and Paseo's relative-time labels.
use super::selected;
use crate::parity::{observe::State, output::Row};
use serde::Serialize;
use std::path::PathBuf;
use vessel_proto::{SessionId, SessionInfo, SessionMode, SessionOption};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AgentRow {
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
    pub(super) fn new(
        info: &SessionInfo,
        status: State,
        thinking: Option<&vessel_proto::OptionValueId>,
        options: &[SessionOption],
    ) -> Self {
        let id = info.id.to_string();
        Self {
            id: info.id,
            short_id: id.chars().take(7).collect(),
            name: info
                .title
                .as_ref()
                .map_or_else(|| "-".to_owned(), ToString::to_string),
            provider: provider(info, options),
            thinking: thinking.map_or_else(|| "auto".to_owned(), ToString::to_string),
            status,
            cwd: info.spec.cwd.clone(),
            created: relative(info.created_at, chrono::Utc::now()),
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
            self.status.label().to_owned(),
            self.cwd
                .as_ref()
                .map_or_else(|| "-".to_owned(), |p| p.display().to_string()),
            self.created.clone(),
        ]
    }
}

fn provider(info: &SessionInfo, options: &[SessionOption]) -> String {
    let model = selected(options, vessel_proto::OptionCategory::Model)
        .map(vessel_proto::OptionValueId::as_str)
        .or_else(|| info.model.as_ref().map(|model| model.model.as_str()))
        .or_else(|| match &info.spec.mode {
            SessionMode::Structured { model, .. } => {
                model.as_ref().map(vessel_proto::ModelId::as_str)
            }
            SessionMode::Terminal { .. } => None,
        })
        .map(str::trim)
        .filter(|model| !model.is_empty() && !model.eq_ignore_ascii_case("default"));
    model.map_or_else(
        || info.spec.harness.to_string(),
        |model| format!("{}/{model}", info.spec.harness),
    )
}
fn relative(created: chrono::DateTime<chrono::Utc>, now: chrono::DateTime<chrono::Utc>) -> String {
    let elapsed = now.signed_duration_since(created);
    let seconds = elapsed.num_seconds();
    if seconds < 60 {
        "just now".to_owned()
    } else if seconds < 3600 {
        format!("{} minutes ago", elapsed.num_minutes())
    } else if seconds < 86400 {
        format!("{} hours ago", elapsed.num_hours())
    } else {
        format!("{} days ago", elapsed.num_days())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_time_boundaries_match_the_reference_without_unsigned_underflow() {
        let now = chrono::Utc::now();
        for (seconds, expected) in [
            (-1, "just now"),
            (59, "just now"),
            (60, "1 minutes ago"),
            (3599, "59 minutes ago"),
            (3600, "1 hours ago"),
            (86400, "1 days ago"),
        ] {
            let created = now
                .checked_sub_signed(chrono::TimeDelta::seconds(seconds))
                .unwrap();
            assert_eq!(relative(created, now), expected);
        }
    }
}
