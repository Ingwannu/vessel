//! Non-terminal provider information retains its meaning above the decoder edge.

use agent_client_protocol::v1::schema as acp;
use vessel_proto::AgentEvent;
use vessel_proto::timeline::{
    Compaction, CompactionStatus, NotificationLevel, TimelineNotification,
};

pub(super) fn notice(notice: acp::Notice) -> AgentEvent {
    let (level, source) = match notice.severity {
        acp::NoticeSeverity::Info => (NotificationLevel::Info, None),
        acp::NoticeSeverity::Warning => (NotificationLevel::Warning, None),
        acp::NoticeSeverity::Error => (NotificationLevel::Error, None),
        acp::NoticeSeverity::Other(source) => (NotificationLevel::Info, Some(source)),
        _ => (NotificationLevel::Info, Some("unknown severity".to_owned())),
    };
    let message = match notice.description {
        Some(description) => format!("{}: {description}", notice.title),
        None => notice.title,
    };
    let message = source.map_or_else(|| message.clone(), |source| format!("{source}: {message}"));
    AgentEvent::Notification(TimelineNotification { level, message })
}

pub(super) fn compaction(update: acp::CompactionUpdate) -> AgentEvent {
    let status = match update.status {
        acp::CompactionStatus::InProgress => CompactionStatus::Loading,
        acp::CompactionStatus::Completed => CompactionStatus::Completed,
        // Paseo's compaction vocabulary cannot express these terminal states.
        // Keep the explicit failure, never fabricate a successful completion.
        acp::CompactionStatus::Failed => {
            return AgentEvent::Error {
                message: "Context compaction failed".into(),
            };
        }
        acp::CompactionStatus::Cancelled => {
            return AgentEvent::Notification(TimelineNotification {
                level: NotificationLevel::Warning,
                message: "Context compaction canceled".into(),
            });
        }
        acp::CompactionStatus::Other(status) => {
            return AgentEvent::Notification(TimelineNotification {
                level: NotificationLevel::Info,
                message: format!("Context compaction status: {status}"),
            });
        }
        _ => {
            return AgentEvent::Notification(TimelineNotification {
                level: NotificationLevel::Info,
                message: "Unknown context compaction status".into(),
            });
        }
    };
    AgentEvent::Compaction(Compaction {
        status,
        trigger: None,
        pre_tokens: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notices_preserve_severity_without_ending_the_turn() {
        for (source, expected) in [
            (acp::NoticeSeverity::Info, NotificationLevel::Info),
            (acp::NoticeSeverity::Warning, NotificationLevel::Warning),
            (acp::NoticeSeverity::Error, NotificationLevel::Error),
        ] {
            let event = notice(acp::Notice::new(source, "Title").description("Detail"));
            assert!(
                matches!(event, AgentEvent::Notification(TimelineNotification { level, message }) if level == expected && message == "Title: Detail")
            );
        }
    }

    #[test]
    fn failed_and_cancelled_compactions_cannot_become_false_successes() {
        for status in [
            acp::CompactionStatus::Failed,
            acp::CompactionStatus::Cancelled,
        ] {
            let event = compaction(acp::CompactionUpdate::new("compaction", status));
            assert!(!matches!(
                event,
                AgentEvent::Compaction(Compaction {
                    status: CompactionStatus::Completed,
                    ..
                })
            ));
        }
        assert!(matches!(
            compaction(acp::CompactionUpdate::new(
                "c",
                acp::CompactionStatus::Completed
            )),
            AgentEvent::Compaction(Compaction {
                status: CompactionStatus::Completed,
                ..
            })
        ));
    }
}
