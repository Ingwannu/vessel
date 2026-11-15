//! Typed log selection, independent of transport and rendering.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use vessel_proto::{AgentEvent, EventRecord, SessionEvent, SessionStatus};

/// An explicit timestamp avoids the machine's locale or timezone changing a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Since(DateTime<Utc>);

impl FromStr for Since {
    type Err = chrono::ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        DateTime::parse_from_rfc3339(text).map(|time| Self(time.with_timezone(&Utc)))
    }
}

/// Closed names make a misspelled filter an input error, never an empty log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Filter {
    /// Tool announcements and updates.
    Tools,
    /// User, assistant and thought content.
    Text,
    /// Failed turns and failed sessions.
    Errors,
    /// Permission requests and their resolution.
    Permissions,
}

/// Predicates applied before the tail window, including on future records.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Selection {
    /// Inclusive timestamp floor, independent of the sequence cursor.
    pub(crate) since: Option<Since>,
    /// Absence preserves every record kind.
    pub(crate) filter: Option<Filter>,
}

impl Selection {
    /// Apply one predicate to history and live records so their boundary is invisible.
    #[must_use]
    pub(crate) fn keeps(self, record: &EventRecord) -> bool {
        self.since.is_none_or(|since| record.at >= since.0)
            && self
                .filter
                .is_none_or(|filter| kind(&record.event) == Some(filter))
    }
}

fn kind(event: &SessionEvent) -> Option<Filter> {
    match event {
        SessionEvent::Agent { event } => agent_kind(event),
        SessionEvent::Status {
            status: SessionStatus::Failed { .. },
        } => Some(Filter::Errors),
        SessionEvent::Status { .. }
        | SessionEvent::Terminal(_)
        | SessionEvent::ProfileChanged { .. }
        | SessionEvent::ModelCall(_)
        | SessionEvent::OptionsChanged { .. }
        | SessionEvent::Renamed { .. }
        | SessionEvent::QueueChanged { .. }
        | SessionEvent::CommandsChanged { .. }
        | SessionEvent::UsageChanged { .. }
        | SessionEvent::Parked { .. }
        | SessionEvent::Woken
        | SessionEvent::Forked { .. }
        | SessionEvent::Rewound { .. } => None,
    }
}

const fn agent_kind(event: &AgentEvent) -> Option<Filter> {
    match event {
        AgentEvent::ToolCall(_) | AgentEvent::ToolCallUpdate(_) => Some(Filter::Tools),
        AgentEvent::TurnStarted { .. }
        | AgentEvent::UserMessageChunk { .. }
        | AgentEvent::MessageChunk { .. }
        | AgentEvent::ThoughtChunk { .. }
        | AgentEvent::Steered { .. } => Some(Filter::Text),
        AgentEvent::TurnFailed { .. } | AgentEvent::Error { .. } => Some(Filter::Errors),
        AgentEvent::PermissionRequested(_) | AgentEvent::PermissionResolved { .. } => {
            Some(Filter::Permissions)
        }
        AgentEvent::Ready { .. }
        | AgentEvent::Plan { .. }
        | AgentEvent::TurnEnded { .. }
        | AgentEvent::Notification(_)
        | AgentEvent::Compaction(_)
        | AgentEvent::Plugin(_)
        | AgentEvent::CommandAccepted { .. }
        | AgentEvent::Error { .. }
        | AgentEvent::HarnessLog { .. }
        | AgentEvent::QuestionAsked(_)
        | AgentEvent::QuestionResolved { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use vessel_proto::{
        ContentBlock, EventSeq, PermissionId, PermissionOutcome, TerminalBytes, TerminalOutput,
    };

    fn record(event: SessionEvent) -> EventRecord {
        EventRecord {
            seq: EventSeq::ZERO,
            at: DateTime::UNIX_EPOCH,
            event,
        }
    }

    #[test]
    fn time_is_inclusive_and_applied_with_the_kind_before_tail_selection() {
        let event = record(SessionEvent::Agent {
            event: AgentEvent::MessageChunk {
                content: ContentBlock::Text {
                    text: "hello".to_owned(),
                },
            },
        });
        let same = Selection {
            since: Some("1970-01-01T01:00:00+01:00".parse().unwrap()),
            filter: Some(Filter::Text),
        };
        assert!(same.keeps(&event));
        assert!(
            !Selection {
                since: Some("1970-01-01T00:00:00.000000001Z".parse().unwrap()),
                ..same
            }
            .keeps(&event)
        );
        assert!(
            !Selection {
                filter: Some(Filter::Tools),
                ..same
            }
            .keeps(&event)
        );
    }

    #[test]
    fn failure_and_permission_kinds_do_not_guess_from_harness_text_or_terminal_bytes() {
        let examples = [
            (
                SessionEvent::Agent {
                    event: AgentEvent::ToolCallUpdate(
                        serde_json::from_str(r#"{"id":"call","status":"failed"}"#).unwrap(),
                    ),
                },
                Some(Filter::Tools),
            ),
            (
                SessionEvent::Status {
                    status: SessionStatus::failed("failure"),
                },
                Some(Filter::Errors),
            ),
            (
                SessionEvent::Agent {
                    event: AgentEvent::TurnFailed {
                        message: "failure".to_owned(),
                    },
                },
                Some(Filter::Errors),
            ),
            (
                SessionEvent::Agent {
                    event: AgentEvent::PermissionResolved {
                        id: PermissionId::random(),
                        outcome: PermissionOutcome::Cancelled,
                    },
                },
                Some(Filter::Permissions),
            ),
            (
                SessionEvent::Agent {
                    event: AgentEvent::HarnessLog {
                        line: "error: denied".to_owned(),
                    },
                },
                None,
            ),
            (
                SessionEvent::Terminal(TerminalOutput {
                    data: TerminalBytes::new(Bytes::from_static(b"error: tool permission")),
                }),
                None,
            ),
        ];
        for (event, expected) in examples {
            assert_eq!(kind(&event), expected, "{event:?}");
            for filter in [
                Filter::Tools,
                Filter::Text,
                Filter::Errors,
                Filter::Permissions,
            ] {
                assert_eq!(
                    Selection {
                        since: None,
                        filter: Some(filter)
                    }
                    .keeps(&record(event.clone())),
                    expected == Some(filter)
                );
            }
        }
    }
}
