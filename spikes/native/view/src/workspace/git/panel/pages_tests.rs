//! Page lineage extends the existing web diff-rows stable-key fixture.
use super::file_target_tests::{load, path, reads};
use super::*;
use crate::workspace::diff_rows::{DiffBody, DiffKey, DiffRow, add_page};
use crate::workspace::git::RequestKeys;
use vessel_proto::workspace::{
    ChangeStatus, DiffContent, DiffLine, FileDiff, Hunk, LineKind, ModeChange,
};

fn page(more_from: Option<u32>) -> FileDiff {
    FileDiff {
        path: path(),
        old_path: Some("old/file.rs".parse().unwrap()),
        status: ChangeStatus::Renamed,
        mode: Some(ModeChange {
            old: 100644,
            new: 100755,
        }),
        content: DiffContent::Hunks {
            hunks: vec![Hunk {
                old_start: 10,
                old_lines: 2,
                new_start: 10,
                new_lines: 2,
                section: Some("fn main()".to_owned()),
                lines: vec![
                    DiffLine {
                        kind: LineKind::Added,
                        text: "한글".to_owned(),
                        cut: true,
                    },
                    DiffLine {
                        kind: LineKind::NoNewline,
                        text: "\\ No newline at end of file".to_owned(),
                        cut: false,
                    },
                ],
            }],
        },
        more_from,
    }
}
fn start(reads: &mut PanelReads, keys: &mut RequestKeys, first: FileDiff) -> FileTicket {
    let target = load(reads, keys);
    let ticket = reads
        .begin_captured_file(&target, keys.issue().unwrap())
        .unwrap();
    assert_eq!(ticket.from_hunk().get(), 0);
    assert!(reads.finish_file(&ticket, Ok(first)));
    ticket
}

#[test]
fn pages_keep_all_file_fields_and_the_web_fixtures_stable_hunk_keys() {
    let mut reads = reads();
    let mut keys = RequestKeys::default();
    let first = start(&mut reads, &mut keys, page(Some(1)));
    let target = reads.next_file_target(&path()).unwrap();
    let next = reads
        .begin_captured_file(&target, keys.issue().unwrap())
        .unwrap();
    assert_eq!(next.from_hunk().get(), 1);
    assert!(!reads.file_target_is_current(&target));
    assert_eq!(
        reads.file(&path()).unwrap().shown().unwrap().file(),
        &page(Some(1))
    );
    assert!(reads.finish_file(&next, Ok(page(None))));
    assert!(!reads.finish_file(&next, Ok(page(None))));
    assert!(!reads.finish_file(&first, Ok(page(Some(1)))));
    let mut expected = page(None);
    let DiffContent::Hunks { hunks } = &mut expected.content else {
        panic!("hunks");
    };
    hunks.push(hunks[0].clone());
    let accepted = reads.file(&path()).unwrap().ready().unwrap();
    assert_eq!(accepted.file(), &expected);
    assert_eq!(accepted.ticket(), &next);
    let DiffBody::Rows { rows, more } = add_page(None, accepted.file()) else {
        panic!("rows");
    };
    assert_eq!(more, None);
    assert_eq!(
        rows.iter().map(DiffRow::key).collect::<Vec<_>>(),
        vec![
            DiffKey::Hunk(0),
            DiffKey::Line { hunk: 0, line: 0 },
            DiffKey::Line { hunk: 0, line: 1 },
            DiffKey::Cut(0),
            DiffKey::Hunk(1),
            DiffKey::Line { hunk: 1, line: 0 },
            DiffKey::Line { hunk: 1, line: 1 },
            DiffKey::Cut(1),
        ]
    );
    assert!(reads.next_file_target(&path()).is_none());
    assert!(!reads.file_target_is_current(&target));
}

#[test]
fn an_equal_refresh_cannot_revive_a_queued_continuation_or_its_answer() {
    let mut reads = reads();
    let mut keys = RequestKeys::default();
    let _first = start(&mut reads, &mut keys, page(Some(1)));
    let target = reads.next_file_target(&path()).unwrap();
    let next = reads
        .begin_captured_file(&target, keys.issue().unwrap())
        .unwrap();
    let _new_summary = load(&mut reads, &mut keys);
    assert!(!reads.file_target_is_current(&target));
    assert!(
        reads
            .begin_captured_file(&target, keys.issue().unwrap())
            .is_none()
    );
    assert!(!reads.finish_file(&next, Ok(page(None))));
    let _new_first = start(&mut reads, &mut keys, page(Some(1)));
    assert!(!reads.file_target_is_current(&target));
}

#[test]
fn malformed_progress_is_a_settled_failure_and_duplicates_cannot_overwrite_it() {
    for more in [0, 2, u32::MAX] {
        let mut reads = reads();
        let mut keys = RequestKeys::default();
        let ticket = start(&mut reads, &mut keys, page(Some(more)));
        assert!(matches!(
            reads.file(&path()),
            Some(PanelRead::Failed(ReadFailure::Page(
                PageMismatch::Continuation
            )))
        ));
        assert!(reads.next_file_target(&path()).is_none());
        assert!(!reads.finish_file(&ticket, Ok(page(Some(1)))));
    }
}

#[test]
fn changed_metadata_or_content_cannot_silently_extend_the_prefix() {
    let base = page(None);
    let changed = [
        FileDiff {
            old_path: None,
            ..base.clone()
        },
        FileDiff {
            status: ChangeStatus::Modified,
            ..base.clone()
        },
        FileDiff {
            mode: None,
            ..base.clone()
        },
        FileDiff {
            content: DiffContent::Binary,
            ..base
        },
    ];
    for (index, changed) in changed.into_iter().enumerate() {
        let mut reads = reads();
        let mut keys = RequestKeys::default();
        let _first = start(&mut reads, &mut keys, page(Some(1)));
        let target = reads.next_file_target(&path()).unwrap();
        let ticket = reads
            .begin_captured_file(&target, keys.issue().unwrap())
            .unwrap();
        assert!(reads.finish_file(&ticket, Ok(changed)));
        let expected = if index == 3 {
            PageMismatch::Content
        } else {
            PageMismatch::Metadata
        };
        assert!(
            matches!(reads.file(&path()), Some(PanelRead::Failed(ReadFailure::Page(why))) if why == &expected)
        );
        assert!(reads.next_file_target(&path()).is_none());
    }
}

#[test]
fn a_failed_continuation_cannot_offer_an_old_ready_prefix_as_a_new_page() {
    let mut reads = reads();
    let mut keys = RequestKeys::default();
    let _first = start(&mut reads, &mut keys, page(Some(1)));
    let target = reads.next_file_target(&path()).unwrap();
    let ticket = reads
        .begin_captured_file(&target, keys.issue().unwrap())
        .unwrap();
    assert!(reads.finish_file(&ticket, Err(ReadFailure::TimedOut)));
    assert!(reads.file(&path()).unwrap().shown().is_none());
    assert!(reads.next_file_target(&path()).is_none());
    assert!(!reads.file_target_is_current(&target));
}

#[test]
fn body_payload_and_empty_line_overhead_are_bounded_before_acceptance() {
    for content in [
        DiffContent::Hunks {
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: 1,
                section: None,
                lines: vec![DiffLine {
                    kind: LineKind::Added,
                    text: "a".repeat(MAX_FILE_TEXT_BYTES),
                    cut: false,
                }],
            }],
        },
        DiffContent::Hunks {
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: 1,
                section: None,
                lines: vec![
                    DiffLine {
                        kind: LineKind::Added,
                        text: String::new(),
                        cut: false
                    };
                    MAX_FILE_LINES.saturating_add(1)
                ],
            }],
        },
    ] {
        let mut reads = reads();
        let mut keys = RequestKeys::default();
        let _ticket = start(
            &mut reads,
            &mut keys,
            FileDiff {
                content,
                ..page(None)
            },
        );
        assert!(matches!(
            reads.file(&path()),
            Some(PanelRead::Failed(ReadFailure::TooLarge))
        ));
    }
}

#[test]
fn an_individually_bounded_page_cannot_grow_the_assembled_file_past_its_limit() {
    let large = |more| {
        let mut file = page(more);
        let DiffContent::Hunks { hunks } = &mut file.content else {
            panic!("hunks");
        };
        hunks[0].lines[0].text = "x".repeat(MAX_FILE_TEXT_BYTES / 2);
        file
    };
    let mut reads = reads();
    let mut keys = RequestKeys::default();
    let _first = start(&mut reads, &mut keys, large(Some(1)));
    assert!(reads.file(&path()).unwrap().ready().is_some());
    let next = reads
        .begin_captured_file(
            &reads.next_file_target(&path()).unwrap(),
            keys.issue().unwrap(),
        )
        .unwrap();
    assert!(reads.finish_file(&next, Ok(large(None))));
    assert!(matches!(
        reads.file(&path()),
        Some(PanelRead::Failed(ReadFailure::TooLarge))
    ));
}

#[test]
fn page_validation_explains_the_mismatch_and_retry_in_both_languages() {
    use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap();
    for reason in [
        PageMismatch::Offset,
        PageMismatch::Continuation,
        PageMismatch::Metadata,
        PageMismatch::Content,
    ] {
        let failure = ReadFailure::Page(reason);
        assert_eq!(ReadIssue::for_failure(&failure), ReadIssue::Page);
        assert!(std::error::Error::source(&failure).is_some());
        assert_eq!(
            ReadIssue::for_failure(&failure).words(en),
            "The received changes do not match. Refresh and try again."
        );
        assert_eq!(
            ReadIssue::for_failure(&failure).words(ko),
            "받은 변경 사항이 서로 맞지 않습니다. 새로고침하고 다시 시도하세요."
        );
    }
}

#[test]
fn two_queued_first_page_callbacks_cannot_supersede_a_read_or_restart_ready_data() {
    let mut reads = reads();
    let mut keys = RequestKeys::default();
    let target = load(&mut reads, &mut keys);
    let first = reads
        .begin_captured_file(&target, keys.issue().unwrap())
        .unwrap();
    assert!(
        reads
            .begin_captured_file(&target, keys.issue().unwrap())
            .is_none()
    );
    assert!(reads.finish_file(&first, Ok(page(Some(1)))));
    assert!(
        reads
            .begin_captured_file(&target, keys.issue().unwrap())
            .is_none()
    );
    let next = reads
        .begin_captured_file(
            &reads.next_file_target(&path()).unwrap(),
            keys.issue().unwrap(),
        )
        .unwrap();
    assert!(reads.finish_file(&next, Err(ReadFailure::TimedOut)));
    let retry = reads
        .begin_captured_file(&target, keys.issue().unwrap())
        .unwrap();
    assert_eq!(retry.from_hunk().get(), 0);
    assert!(reads.finish_file(&retry, Ok(page(None))));
}
