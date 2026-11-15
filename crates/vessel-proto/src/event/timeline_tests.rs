//! Pending event kinds use readable stand-ins along the same durable cursor path.

use super::*;
use crate::timeline::{
    Compaction, CompactionStatus, NotificationLevel, PluginTimelineItem, TimelineNotification,
};

#[test]
fn new_timeline_items_cannot_escape_the_unassigned_protocol_gate() {
    let plugin: PluginTimelineItem = serde_json::from_str(
        r#"{"id":"item","plugin":"plugin","kind":"card","version":1,"data":{"private":"content"}}"#,
    )
    .unwrap();
    for event in [
        AgentEvent::Notification(TimelineNotification {
            level: NotificationLevel::Warning,
            message: "notice".into(),
        }),
        AgentEvent::Compaction(Compaction {
            status: CompactionStatus::Loading,
            trigger: None,
            pre_tokens: None,
        }),
        AgentEvent::Plugin(plugin),
        AgentEvent::Error {
            message: "non-terminal".into(),
        },
    ] {
        assert_eq!(event.required_protocol(), crate::timeline::TIMELINE_PARITY);
        let encoded = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<AgentEvent>(&encoded).unwrap(), event);
        for version in [crate::MIN_PROTOCOL_VERSION, crate::PROTOCOL_VERSION] {
            let record = EventRecord {
                seq: EventSeq::new(7),
                at: DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent {
                    event: event.clone(),
                },
            };
            let adapted = record
                .for_protocol(version)
                .expect("unassigned event cannot be sent");
            assert_eq!(adapted.seq, record.seq);
            assert_eq!(adapted.at, record.at);
            assert!(matches!(
                adapted.event,
                SessionEvent::Agent {
                    event: AgentEvent::HarnessLog { .. }
                }
            ));
            assert!(!serde_json::to_string(&adapted).unwrap().contains("private"));
            let page = crate::EventPage {
                records: vec![record],
                next: EventSeq::new(8),
                at_end: true,
            };
            assert_eq!(page.for_protocol(version).unwrap().records, vec![adapted]);
        }
    }
}
