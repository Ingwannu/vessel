//! Native system progress has a lifetime independent of a model turn.
use super::Translator;
use crate::claude::wire::{
    System,
    timeline::{StatusKind, Trigger},
};
use vessel_proto::{
    AgentEvent,
    timeline::{
        Compaction, CompactionStatus, CompactionTrigger, NotificationLevel, TimelineNotification,
    },
};

impl Translator {
    pub(crate) fn system(&mut self, system: &System) -> Vec<AgentEvent> {
        let event = match system {
            System::Status(status)
                if status.status == Some(StatusKind::Compacting) && !self.compact_open =>
            {
                self.compact_open = true;
                AgentEvent::Compaction(Compaction {
                    status: CompactionStatus::Loading,
                    trigger: None,
                    pre_tokens: None,
                })
            }
            System::CompactBoundary(boundary) => {
                self.compact_open = false;
                let trigger = boundary
                    .compact_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.trigger)
                    .and_then(|trigger| match trigger {
                        Trigger::Auto => Some(CompactionTrigger::Auto),
                        Trigger::Manual => Some(CompactionTrigger::Manual),
                        Trigger::Other => None,
                    });
                AgentEvent::Compaction(Compaction {
                    status: CompactionStatus::Completed,
                    trigger,
                    pre_tokens: boundary
                        .compact_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.pre_tokens),
                })
            }
            System::ApiRetry(retry) => AgentEvent::Notification(TimelineNotification {
                level: NotificationLevel::Warning,
                message: format!(
                    "Claude is retrying an API request ({}/{})",
                    retry.attempt, retry.max_retries
                ),
            }),
            System::PermissionDenied(denied) => AgentEvent::Notification(TimelineNotification {
                level: NotificationLevel::Warning,
                message: format!("Claude declined {}", denied.tool_name),
            }),
            System::Init(_) | System::Status(_) | System::Unknown => return vec![],
        };
        vec![event]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn decode(value: serde_json::Value) -> System {
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn repeated_native_status_opens_once_and_boundary_keeps_reported_metadata() {
        let mut translator = Translator::default();
        let loading = decode(json!({"subtype":"status","status":"compacting"}));
        assert!(matches!(
            translator.system(&loading).as_slice(),
            [AgentEvent::Compaction(Compaction {
                status: CompactionStatus::Loading,
                ..
            })]
        ));
        assert!(translator.system(&loading).is_empty());
        let boundary = decode(
            json!({"subtype":"compact_boundary","compact_metadata":{"trigger":"manual","pre_tokens":1234}}),
        );
        assert!(
            matches!(translator.system(&boundary).as_slice(), [AgentEvent::Compaction(Compaction {status: CompactionStatus::Completed,trigger:Some(CompactionTrigger::Manual),pre_tokens:Some(tokens)})] if tokens.get()==1234)
        );
        assert_eq!(translator.system(&loading).len(), 1);
        let missing = decode(json!({"subtype":"compact_boundary"}));
        assert!(matches!(
            translator.system(&missing).as_slice(),
            [AgentEvent::Compaction(Compaction {
                trigger: None,
                pre_tokens: None,
                ..
            })]
        ));
    }
}
