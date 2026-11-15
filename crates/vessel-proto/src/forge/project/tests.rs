//! Adversarial page/cursor decoding and the value-only protocol boundary.

use serde::Serialize;

use super::*;
use crate::forge::{ChecksState, ForgeKind, Mergeable, PullNumber, PullTitle, ReviewDecision};

fn repo() -> RepoRef {
    RepoRef {
        kind: ForgeKind::Github,
        host: "forge.example.invalid".parse().unwrap(),
        path: "fixture/project".parse().unwrap(),
    }
}

fn pull(number: usize) -> PullSummary {
    let number = PullNumber::new(u64::try_from(number).unwrap()).unwrap();
    PullSummary {
        repo: repo(),
        number,
        url: format!("https://forge.example.invalid/fixture/project/pull/{number}")
            .parse()
            .unwrap(),
        title: PullTitle::lossy("A project change, independent of a session"),
        state: PullState::Open,
        draft: false,
        head_commit: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
        checks: ChecksState::Failing,
        review: ReviewDecision::ChangesRequested,
        mergeable: Mergeable::Blocked,
        updated_at: DateTime::<Utc>::UNIX_EPOCH,
    }
}

// Serialize intentionally unchecked input so invalid page states reach
// the real deserializer without an untyped JSON tree or a valid-page
// constructor making the test pass before decoding even happens.
#[derive(Serialize)]
struct Input<'a> {
    repo: RepoRef,
    pulls: &'a [PullSummary],
    next: Option<PullCursor>,
    read_at: DateTime<Utc>,
}

fn decode(
    pulls: &[PullSummary],
    next: Option<PullCursor>,
) -> Result<OpenPullPage, serde_json::Error> {
    serde_json::from_str(&serde_json::to_string(&Input {
        repo: repo(),
        pulls,
        next,
        read_at: DateTime::<Utc>::UNIX_EPOCH,
    })?)
}

#[test]
fn cursor_boundaries_and_json_decoding_preserve_opaque_text() {
    for text in [
        "a".to_owned(),
        "A+/=_:-"
            .repeat(PullCursor::MAX_BYTES)
            .chars()
            .take(PullCursor::MAX_BYTES)
            .collect(),
    ] {
        let cursor: PullCursor = text.parse().unwrap();
        assert_eq!(cursor.as_str(), text);
        assert_eq!(
            serde_json::from_str::<PullCursor>(&serde_json::to_string(&cursor).unwrap()).unwrap(),
            cursor
        );
    }
    assert_eq!("".parse::<PullCursor>(), Err(InvalidPullCursor::Empty));
    let long = "a".repeat(PullCursor::MAX_BYTES.checked_add(1).unwrap());
    assert_eq!(long.parse::<PullCursor>(), Err(InvalidPullCursor::TooLong));
    for text in [
        "line\nbreak",
        "\r",
        "has space",
        "\t",
        "\0",
        "\u{7f}",
        "한글",
        "é",
    ] {
        assert_eq!(
            text.parse::<PullCursor>(),
            Err(InvalidPullCursor::Characters)
        );
        let encoded = serde_json::to_string(text).unwrap();
        assert!(serde_json::from_str::<PullCursor>(&encoded).is_err());
    }
    assert!(serde_json::from_str::<PullCursor>(&serde_json::to_string(&long).unwrap()).is_err());
    for json in ["null", "false", "1", "[]", "{}"] {
        assert!(serde_json::from_str::<PullCursor>(json).is_err());
    }
}

#[test]
fn cursor_debug_and_errors_do_not_disclose_provider_metadata() {
    let cursor: PullCursor = "private-provider-metadata".parse().unwrap();
    assert_eq!(format!("{cursor:?}"), "PullCursor(..)");
    let invalid = "private-provider-metadata\n"
        .parse::<PullCursor>()
        .unwrap_err();
    assert_eq!(
        invalid.to_string(),
        "a pull-request cursor must contain only visible ASCII characters"
    );
    let associated = ProjectPullCursor {
        repo: repo(),
        after: cursor,
    };
    assert!(!format!("{associated:?}").contains("private-provider-metadata"));
}

#[test]
fn empty_means_successfully_read_nothing_and_has_no_continuation() {
    let empty = decode(&[], None).unwrap();
    assert_eq!(empty.repo(), &repo());
    assert_eq!(empty.pulls(), []);
    assert_eq!(empty.continuation(), None);
    assert_eq!(empty.read_at(), DateTime::<Utc>::UNIX_EPOCH);
    assert_eq!(
        serde_json::from_str::<OpenPullPage>(&serde_json::to_string(&empty).unwrap()).unwrap(),
        empty
    );
}

#[test]
fn a_full_page_keeps_facts_order_and_its_repository_bound_continuation() {
    let pulls: Vec<_> = (1..=OpenPullPage::MAX_PULLS).rev().map(pull).collect();
    let after: PullCursor = "provider-cursor".parse().unwrap();
    let page = decode(&pulls, Some(after.clone())).unwrap();
    assert_eq!(page.pulls(), pulls);
    assert_eq!(
        page.continuation(),
        Some(ProjectPullCursor {
            repo: repo(),
            after
        })
    );
    let json = serde_json::to_string(&page).unwrap();
    assert_eq!(serde_json::from_str::<OpenPullPage>(&json).unwrap(), page);
    let cursor = page.continuation().unwrap();
    let json = serde_json::to_string(&cursor).unwrap();
    assert_eq!(
        serde_json::from_str::<ProjectPullCursor>(&json).unwrap(),
        cursor
    );
}

#[test]
fn constructor_and_decoder_refuse_each_inconsistent_page_whole() {
    let at = DateTime::<Utc>::UNIX_EPOCH;
    let after: PullCursor = "continuation".parse().unwrap();
    assert_eq!(
        OpenPullPage::new(repo(), vec![], Some(after.clone()), at),
        Err(InvalidOpenPullPage::EmptyContinuation)
    );
    assert!(decode(&[], Some(after)).is_err());
    let many: Vec<_> = (1..=OpenPullPage::MAX_PULLS.checked_add(1).unwrap())
        .map(pull)
        .collect();
    assert_eq!(
        OpenPullPage::new(repo(), many.clone(), None, at),
        Err(InvalidOpenPullPage::TooMany)
    );
    assert!(decode(&many, None).is_err());
    let duplicated = vec![pull(1), pull(1)];
    assert_eq!(
        OpenPullPage::new(repo(), duplicated.clone(), None, at),
        Err(InvalidOpenPullPage::Duplicate)
    );
    assert!(decode(&duplicated, None).is_err());
    let mut foreign = pull(1);
    foreign.repo.path = "another/repository".parse().unwrap();
    assert_eq!(
        OpenPullPage::new(repo(), vec![foreign.clone()], None, at),
        Err(InvalidOpenPullPage::Repository)
    );
    assert!(decode(&[foreign], None).is_err());
    for state in [PullState::Closed, PullState::Merged, PullState::Unknown] {
        let mut not_open = pull(1);
        not_open.state = state;
        assert_eq!(
            OpenPullPage::new(repo(), vec![not_open.clone()], None, at),
            Err(InvalidOpenPullPage::NotOpen)
        );
        assert!(decode(&[not_open], None).is_err());
    }
}

#[test]
fn an_oversized_array_refuses_before_decoding_another_summary() {
    let full: Vec<_> = (1..=OpenPullPage::MAX_PULLS).map(pull).collect();
    let encoded = serde_json::to_string(&full).unwrap();
    let prefix = encoded.strip_suffix(']').unwrap();
    let overflow = format!(r#"{prefix},"this is not a summary"]"#);
    let mut input = serde_json::Deserializer::from_str(&overflow);
    let error = bounded_pulls(&mut input).unwrap_err();
    let position = format!(" at line {} column {}", error.line(), error.column());
    assert_eq!(error.classify(), serde_json::error::Category::Data);
    assert_eq!(
        error.to_string().strip_suffix(&position),
        Some(
            format!(
                "an open pull-request page exceeds {} items",
                OpenPullPage::MAX_PULLS
            )
            .as_str()
        )
    );
}

#[test]
fn value_types_do_not_make_a_project_list_call_decodable() {
    assert!(serde_json::from_str::<crate::Method>(r#""pr.project_list""#).is_err());
    assert!(
        serde_json::from_str::<crate::Call>(r#"{"method":"pr.project_list","params":{}}"#).is_err()
    );
}
