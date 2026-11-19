//! An agent's Diagnostic sheet and the node logs' contents (ADR 0111): what
//! each shows, and that an answer for another ask is dropped.

use std::path::PathBuf;

use vessel_proto::ErrorCode;
use vessel_proto::diagnostics::{LogSink, NodeLog};

use super::{LogContents, Problem, Sheet, SheetBody, log_contents};
use crate::devices::Tickets;

#[test]
fn a_sheet_runs_then_shows_its_text_and_keeps_it_while_refreshing() {
    let mut tickets = Tickets::default();
    let first = tickets.next_ticket().unwrap();
    let mut sheet = Sheet::asking(first);
    assert_eq!(sheet.body(), SheetBody::Running);
    assert_eq!(sheet.text(), None);
    let other = tickets.next_ticket().unwrap();
    assert!(!sheet.answered(other, Ok("not this one".to_owned())));
    assert!(sheet.answered(first, Ok("Claude Code\n  Status: Ready".to_owned())));
    assert!(!sheet.loading());
    assert_eq!(
        sheet.body(),
        SheetBody::Text("Claude Code\n  Status: Ready")
    );
    let again = tickets.next_ticket().unwrap();
    sheet.refresh(again);
    assert!(sheet.loading());
    assert_eq!(
        sheet.text(),
        Some("Claude Code\n  Status: Ready"),
        "kept while it asks"
    );
    assert!(sheet.answered(again, Err(Problem::Refused(ErrorCode::Unavailable))));
    assert_eq!(
        sheet.body(),
        SheetBody::Failed(Problem::Refused(ErrorCode::Unavailable))
    );
    assert_eq!(sheet.text(), None, "a failure is not copied as text");
    assert_eq!(Sheet::default().body(), SheetBody::None);
}

#[test]
fn the_logs_say_empty_only_for_an_empty_file_and_a_failure_as_one() {
    let file = |lines: &[&str]| NodeLog::File {
        path: PathBuf::from("~/.vessel/logs/vesseld.log"),
        lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        cut: false,
    };
    assert_eq!(log_contents(&Ok(file(&[]))), LogContents::Empty);
    assert_eq!(
        log_contents(&Ok(file(&["one", "two"]))),
        LogContents::Lines("one\ntwo".to_owned())
    );
    assert_eq!(
        log_contents(&Ok(NodeLog::NoFile {
            writes_to: LogSink::Terminal
        })),
        LogContents::None
    );
    assert_eq!(
        log_contents(&Err(Problem::Offline)),
        LogContents::Error(Problem::Offline)
    );
}
