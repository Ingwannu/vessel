use super::*;
use vessel_proto::ProjectId;

fn key(machine: &str) -> ReviewKey {
    ReviewKey {
        machine: machine.to_owned(),
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
        mode: ReviewMode::Uncommitted,
        base: None,
        ignore_whitespace: false,
    }
}

fn target(line: u32) -> ReviewTarget {
    ReviewTarget {
        path: "src/upload/retry.rs".parse().unwrap(),
        side: ReviewSide::New,
        line: NonZeroU32::new(line).unwrap(),
    }
}

fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_795_000_000_i64.saturating_add(seconds), 0).unwrap()
}

#[test]
fn a_comment_is_added_then_edited_in_place_with_a_new_time() {
    let mut drafts = ReviewDrafts::default();
    let review = key("laptop");
    let Saved::Added(id) = drafts.save(&review, None, target(48), "  Cap this.  ", at(0)) else {
        panic!("added");
    };
    assert_eq!(drafts.comments(&review).len(), 1);
    assert_eq!(drafts.comments(&review)[0].body, "Cap this.");
    assert_eq!(
        drafts.save(&review, Some(id), target(48), "Cap this at 30 s.", at(5)),
        Saved::Edited(id)
    );
    let comment = &drafts.comments(&review)[0];
    assert_eq!(comment.body, "Cap this at 30 s.");
    assert_eq!((comment.created_at, comment.updated_at), (at(0), at(5)));
    // An identity no longer kept (sent meanwhile) saves a new comment.
    let _done = drafts.delete(&review, id);
    assert!(matches!(
        drafts.save(&review, Some(id), target(48), "Again.", at(6)),
        Saved::Added(other) if other != id
    ));
}

#[test]
fn a_blank_or_too_long_body_is_not_saved() {
    let mut drafts = ReviewDrafts::default();
    let review = key("laptop");
    assert_eq!(
        drafts.save(&review, None, target(1), " \n ", at(0)),
        Saved::Blank
    );
    let long = "w".repeat(BODY_BYTES.saturating_add(1));
    assert_eq!(
        drafts.save(&review, None, target(1), &long, at(0)),
        Saved::TooLong
    );
    assert!(!can_save(&long));
    assert!(can_save(&"w".repeat(BODY_BYTES)));
    assert!(drafts.is_empty());
}

#[test]
fn comments_are_kept_per_key_and_per_line() {
    let mut drafts = ReviewDrafts::default();
    let (one, two) = (key("laptop"), key("build-server"));
    let _done = drafts.save(&one, None, target(48), "first", at(0));
    let _done = drafts.save(&one, None, target(48), "second", at(1));
    let _done = drafts.save(&one, None, target(50), "elsewhere", at(2));
    let _done = drafts.save(&two, None, target(48), "other machine", at(3));
    let on: Vec<&str> = drafts
        .on(&one, &target(48))
        .map(|comment| comment.body.as_str())
        .collect();
    assert_eq!(on, ["first", "second"]);
    assert_eq!(drafts.comments(&two).len(), 1);
    let mut staged = one.clone();
    staged.mode = ReviewMode::Staged;
    assert!(drafts.comments(&staged).is_empty());
}

#[test]
fn only_the_sent_versions_are_cleared() {
    let mut drafts = ReviewDrafts::default();
    let review = key("laptop");
    let _done = drafts.save(&review, None, target(48), "kept as sent", at(0));
    let Saved::Added(edited) = drafts.save(&review, None, target(49), "before", at(1)) else {
        panic!("added");
    };
    let sent = drafts.comments(&review).to_vec();
    // Edited while the review was on its way: that version stays.
    let _done = drafts.save(&review, Some(edited), target(49), "after", at(2));
    let _done = drafts.save(&review, None, target(50), "written after", at(3));
    assert_eq!(drafts.clear_sent(&review, &sent), 1);
    let left: Vec<&str> = drafts
        .comments(&review)
        .iter()
        .map(|comment| comment.body.as_str())
        .collect();
    assert_eq!(left, ["after", "written after"]);
    drafts.clear(&review);
    assert!(drafts.is_empty());
}

#[test]
fn the_bounds_drop_the_oldest_first() {
    let mut drafts = ReviewDrafts::default();
    let review = key("laptop");
    for n in 0..=ReviewDrafts::MAX_COMMENTS {
        let _done = drafts.save(&review, None, target(1), &format!("c{n}"), at(0));
    }
    let comments = drafts.comments(&review);
    assert_eq!(comments.len(), ReviewDrafts::MAX_COMMENTS);
    assert_eq!(comments[0].body, "c1");
    let first = key("first");
    let _done = drafts.save(&first, None, target(1), "oldest key", at(0));
    for n in 0..ReviewDrafts::MAX_KEYS {
        let _done = drafts.save(&key(&format!("m{n}")), None, target(1), "x", at(1));
    }
    assert!(drafts.comments(&first).is_empty());
    assert!(drafts.comments(&review).is_empty());
    assert_eq!(drafts.reviews.len(), ReviewDrafts::MAX_KEYS);
    // The bytes in all: the key changed longest ago goes.
    let mut big = ReviewDrafts::default();
    let body = "b".repeat(BODY_BYTES);
    let count = ReviewDrafts::MAX_BYTES.checked_div(BODY_BYTES).unwrap();
    let early = key("early");
    let _done = big.save(&early, None, target(1), &body, at(0));
    for n in 0..count {
        let _done = big.save(&key(&format!("k{n}")), None, target(1), &body, at(1));
    }
    assert!(big.comments(&early).is_empty());
    assert_eq!(big.reviews.len(), count);
}

#[test]
fn the_document_keeps_its_shape_and_prunes_what_it_reads() {
    let mut drafts = ReviewDrafts::default();
    let review = key("laptop");
    let _done = drafts.save(&review, None, target(48), "Cap this.", at(0));
    let json = serde_json::to_value(&drafts).unwrap();
    let comment = &json[0]["comments"][0];
    assert_eq!(comment["path"], "src/upload/retry.rs");
    assert_eq!(comment["side"], "new");
    assert_eq!(comment["line"], 48);
    assert_eq!(json[0]["key"]["mode"], "uncommitted");
    let mut read: ReviewDrafts = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(read, drafts);
    let mut blank = json;
    blank[0]["comments"][0]["body"] = serde_json::json!("  ");
    read = serde_json::from_value(blank).unwrap();
    read.prune();
    assert!(read.is_empty());
}

mod snapshots {
    use super::*;
    use crate::workspace::diff_rows::{DiffKey, DiffRow};
    use std::sync::Arc;
    use vessel_proto::prompt_context::{MAX_COMMENTS, ReviewLineKind};
    use vessel_proto::workspace::LineKind;

    fn row(line: u32, kind: LineKind, old: Option<u32>, new: Option<u32>) -> DiffRow {
        DiffRow::Line {
            key: DiffKey::Line { hunk: 0, line },
            line: kind,
            text: Arc::from(format!("line {line}\r").as_str()),
            cut: false,
            old,
            new,
        }
    }

    /// One hunk: context 40-45, two added lines (new 46, 47), removed
    /// old 46, context to old 50 / new 51.
    fn rows() -> Vec<DiffRow> {
        let mut rows = vec![DiffRow::Hunk {
            key: DiffKey::Hunk(0),
            header: "@@ -40,11 +40,12 @@".to_owned(),
            section: Some("fn backoff".to_owned()),
        }];
        for n in 0..6 {
            rows.push(row(
                n,
                LineKind::Context,
                Some(n.saturating_add(40)),
                Some(n.saturating_add(40)),
            ));
        }
        rows.push(row(6, LineKind::Added, None, Some(46)));
        rows.push(row(7, LineKind::Added, None, Some(47)));
        rows.push(row(8, LineKind::Removed, Some(46), None));
        for n in 0..4 {
            rows.push(row(
                n.saturating_add(9),
                LineKind::Context,
                Some(n.saturating_add(47)),
                Some(n.saturating_add(48)),
            ));
        }
        rows
    }

    fn comment(side: ReviewSide, line: u32, body: &str) -> DraftComment {
        DraftComment {
            id: CommentId(u64::from(line)),
            target: ReviewTarget {
                path: "src/upload/retry.rs".parse().unwrap(),
                side,
                line: NonZeroU32::new(line).unwrap(),
            },
            body: body.to_owned(),
            created_at: at(0),
            updated_at: at(0),
        }
    }

    #[test]
    fn a_comment_carries_its_line_and_three_either_side_of_its_hunk() {
        let rows = rows();
        let review = key("laptop");
        let comments = [
            comment(ReviewSide::New, 47, "Cap this."),
            comment(ReviewSide::Old, 46, "Why removed?"),
        ];
        let made = snapshot("/work/fleet-agent", &review, &comments, |_| Some(&rows)).unwrap();
        assert_eq!(made.sent, comments.to_vec());
        assert_eq!(made.review.root.as_str(), "/work/fleet-agent");
        let first = &made.review.comments[0];
        assert_eq!(first.hunk_header.as_str(), "@@ -40,11 +40,12 @@ fn backoff");
        assert_eq!(first.target.new, NonZeroU32::new(47));
        assert_eq!(first.target.content.as_str(), "line 7");
        assert_eq!(first.lines.len(), 7);
        assert_eq!(first.lines[0].new, NonZeroU32::new(44));
        let second = &made.review.comments[1];
        assert_eq!(second.target.old, NonZeroU32::new(46));
        assert_eq!(second.target.kind, ReviewLineKind::Remove);
        assert!(
            made.text()
                .starts_with("Vessel review attachment (uncommitted)\n")
        );
    }

    #[test]
    fn comments_off_the_diff_or_on_a_file_not_read_are_left_out() {
        let rows = rows();
        let review = key("laptop");
        let gone = comment(ReviewSide::New, 90, "Line left the diff.");
        assert_eq!(
            snapshot("/work", &review, std::slice::from_ref(&gone), |_| Some(
                &rows
            )),
            None
        );
        let kept = comment(ReviewSide::New, 46, "Kept.");
        let made = snapshot("/work", &review, &[gone, kept.clone()], |_| Some(&rows)).unwrap();
        assert_eq!(made.sent, vec![kept.clone()]);
        assert_eq!(snapshot("/work", &review, &[kept], |_| None), None);
    }

    #[test]
    fn a_review_holds_at_most_the_wires_count_of_comments() {
        let rows = rows();
        let review = key("laptop");
        let many: Vec<DraftComment> = (0..MAX_COMMENTS.saturating_add(5))
            .map(|_| comment(ReviewSide::New, 46, "Again."))
            .collect();
        let made = snapshot("/work", &review, &many, |_| Some(&rows)).unwrap();
        assert_eq!(made.review.comments.len(), MAX_COMMENTS);
        assert_eq!(made.sent.len(), MAX_COMMENTS);
    }
}
