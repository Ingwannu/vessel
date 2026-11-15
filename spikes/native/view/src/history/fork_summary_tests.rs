//! Synthetic timeline rows exercise UTF-16 bounds and stable fork selection.

use super::*;
use vessel_proto::{EventSeq, NodeId, SessionId};

fn source() -> SessionRef {
    SessionRef {
        node: NodeId::random(),
        session: SessionId::random(),
    }
}
fn metadata() -> SummarySource<'static> {
    SummarySource {
        title: None,
        directory: None,
        history: SummaryHistory::Complete,
    }
}
fn entry(seq: u64, kind: EntryKind) -> Entry {
    Entry {
        key: EntryKey::Record(EventSeq::new(seq)),
        seq: EventSeq::new(seq),
        at: chrono::DateTime::UNIX_EPOCH,
        kind,
        rev: 0,
    }
}
fn summary(entries: &[Entry]) -> ForkSummary {
    let source = source();
    let rows: Vec<_> = entries
        .iter()
        .map(|entry| SummaryEntry { source, entry })
        .collect();
    fork_summary(&rows, None, metadata()).unwrap()
}
fn message(text: String) -> EntryKind {
    EntryKind::Message { text }
}

#[test]
fn complete_history_under_budget_preserves_every_message() {
    let text = "Complete conversation ".to_owned() + &"x".repeat(90_000);
    let result = summary(&[entry(0, message(text.clone()))]);
    assert_eq!(
        result.as_str(),
        format!("{OPEN}\n\n[Assistant] {text}{CLOSE}")
    );
    assert!(!result.limited());
    assert_eq!(result.history(), SummaryHistory::Complete);
}

#[test]
fn overflow_keeps_recent_whole_messages_before_older_ones() {
    let entries = [
        entry(0, message("old ".to_owned() + &"o".repeat(70_000))),
        entry(1, message("recent ".to_owned() + &"r".repeat(40_000))),
    ];
    let result = summary(&entries);
    assert!(result.as_str().contains(OMITTED));
    assert!(result.as_str().contains("[Assistant] recent"));
    assert!(!result.as_str().contains("[Assistant] old"));
    assert!(result.limited());
    assert!(result.as_str().encode_utf16().count() <= MAX_FORK_UNITS);
}

#[test]
fn an_oversized_message_preserves_both_ends_without_splitting_unicode() {
    let result = summary(&[entry(
        0,
        message("Beginning. ".to_owned() + &"😀".repeat(110_000) + " The end."),
    )]);
    assert!(result.as_str().contains("[Assistant] Beginning."));
    assert!(result.as_str().contains(CUT));
    assert!(result.as_str().contains("The end."));
    assert!(result.as_str().ends_with(CLOSE));
    assert!(result.as_str().encode_utf16().count() <= MAX_FORK_UNITS);
    assert!(result.as_str().encode_utf16().count() >= MAX_FORK_UNITS.saturating_sub(2));
}

#[test]
fn reserving_an_omission_notice_never_drops_the_only_recent_message() {
    let overhead = format!("{OPEN}\n\n{CLOSE}").encode_utf16().count();
    let body = "Beginning ".to_owned()
        + &"x".repeat(MAX_FORK_UNITS.saturating_sub(overhead).saturating_sub(40))
        + " End";
    let result = summary(&[
        entry(0, message("old".repeat(100))),
        entry(1, message(body)),
    ]);
    assert!(result.as_str().contains("Beginning"));
    assert!(result.as_str().contains("End"));
    assert!(result.as_str().contains(OMITTED));
    assert!(result.as_str().encode_utf16().count() <= MAX_FORK_UNITS);
}

#[test]
fn unavailable_history_is_not_an_empty_success_and_retention_is_disclosed() {
    let unavailable = SummarySource {
        history: SummaryHistory::Unavailable,
        ..metadata()
    };
    assert_eq!(
        fork_summary(&[], None, unavailable),
        Err(SummaryProblem::Unavailable)
    );
    let partial = SummarySource {
        history: SummaryHistory::RetainedOnly,
        ..metadata()
    };
    let result = fork_summary(&[], None, partial).unwrap();
    assert_eq!(result.history(), SummaryHistory::RetainedOnly);
    assert!(result.as_str().contains(UNAVAILABLE));
    assert!(!result.limited());
}

#[test]
fn boundary_identity_contains_the_source_and_detects_changed_rows() {
    let a = source();
    let b = source();
    let first = entry(0, message("first".into()));
    let mut second = entry(0, message("second".into()));
    let boundary = ForkBoundary::of(b, &second);
    let rows = [
        SummaryEntry {
            source: a,
            entry: &first,
        },
        SummaryEntry {
            source: b,
            entry: &second,
        },
    ];
    let result = fork_summary(&rows, Some(&boundary), metadata()).unwrap();
    assert!(result.as_str().contains("first"));
    assert!(result.as_str().contains("second"));
    assert_eq!(
        fork_summary(&rows[..1], Some(&boundary), metadata()),
        Err(SummaryProblem::StaleBoundary)
    );
    second.rev = second.rev.saturating_add(1);
    let changed = [SummaryEntry {
        source: b,
        entry: &second,
    }];
    assert_eq!(
        fork_summary(&changed, Some(&boundary), metadata()),
        Err(SummaryProblem::StaleBoundary)
    );
}

#[test]
fn thoughts_logs_and_opaque_tool_arguments_are_excluded() {
    use crate::timeline::ToolView;
    use vessel_proto::{ToolCall, ToolCallId, ToolCallStatus, ToolKind};
    let call = ToolCall {
        id: ToolCallId::new("call"),
        title: "Read file".into(),
        kind: ToolKind::Read,
        status: ToolCallStatus::Completed,
        content: vec![],
        locations: vec![],
        raw_input: Some(serde_json::from_str(r#"{"excluded_raw_payload":true}"#).unwrap()),
    };
    let entries = [
        entry(0, EntryKind::Tool(ToolView::announced(&call))),
        entry(
            1,
            EntryKind::Thought {
                text: "hidden thought".into(),
            },
        ),
        entry(
            2,
            EntryKind::Log {
                line: "hidden diagnostic".into(),
            },
        ),
    ];
    let result = summary(&entries);
    assert!(result.as_str().contains("[Tool] Read file"));
    assert!(!result.as_str().contains("excluded_raw_payload"));
    assert!(!result.as_str().contains("hidden"));
}

#[test]
fn source_metadata_is_bounded_inside_the_same_total_budget() {
    let directory = "/".to_owned() + &"한".repeat(200_000);
    let path = Path::new(&directory);
    let source = SummarySource {
        directory: Some(path),
        ..metadata()
    };
    let result = fork_summary(&[], None, source).unwrap();
    assert!(result.as_str().contains("Source directory: /"));
    assert!(result.as_str().encode_utf16().count() <= MAX_FORK_UNITS);
    assert!(result.as_str().encode_utf16().count() < MAX_METADATA_UNITS.saturating_add(200));
}
