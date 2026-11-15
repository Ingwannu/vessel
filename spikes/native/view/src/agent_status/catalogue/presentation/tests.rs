use super::*;

#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Offline,
}
fn fetched() -> Catalogue {
    serde_json::from_str(include_str!("../fixtures/catalogue.json")).unwrap()
}
fn show<'a>(read: Read<'a, Failure>, query: &str) -> Presentation<'a, Failure> {
    present(read, query)
}

#[test]
fn unfetched_empty_loading_and_first_failure_are_different_bodies() {
    let never = Catalogue::default();
    let failure = Failure::Offline;
    assert!(matches!(
        show(Read::NotRequested, "").body,
        Body::NotRequested
    ));
    assert!(matches!(
        show(Read::Ready(&never), "").body,
        Body::NotRequested
    ));
    let reading = show(
        Read::Pending {
            previous: Some(&never),
        },
        "",
    );
    assert!(matches!(reading.body, Body::Loading));
    assert_eq!(reading.activity, Activity::Pending);
    assert!(reading.retained_failure.is_none());
    for previous in [None, Some(&never)] {
        let failed = show(
            Read::Failed {
                error: &failure,
                previous,
            },
            "",
        );
        assert!(matches!(failed.body, Body::Failed(Failure::Offline)));
        assert!(
            failed.retained_failure.is_none(),
            "the failure is said only once"
        );
        assert_eq!(failed.activity, Activity::Idle);
    }
    let mut empty = fetched();
    empty.entries.clear();
    assert!(matches!(show(Read::Ready(&empty), "").body, Body::Empty));
    assert!(matches!(
        show(
            Read::Pending {
                previous: Some(&empty)
            },
            ""
        )
        .body,
        Body::Loading
    ));
    let failed = show(
        Read::Failed {
            error: &failure,
            previous: Some(&empty),
        },
        "",
    );
    assert!(matches!(failed.body, Body::Empty));
    assert_eq!(failed.retained_failure, Some(&failure));
}

#[test]
fn usable_cached_rows_remain_while_refreshing_or_after_failure() {
    let copy = fetched();
    let failure = Failure::Offline;
    for (read, activity, expected_failure) in [
        (Read::Ready(&copy), Activity::Idle, None),
        (
            Read::Pending {
                previous: Some(&copy),
            },
            Activity::Pending,
            None,
        ),
        (
            Read::Failed {
                error: &failure,
                previous: Some(&copy),
            },
            Activity::Idle,
            Some(&failure),
        ),
    ] {
        let shown = show(read, "example");
        let Body::Rows { first, rest } = shown.body else {
            panic!("cached rows were hidden")
        };
        assert_eq!(first.id.as_str(), "example-agent");
        assert_eq!(first.version.as_str(), "1.2.3");
        assert!(rest.is_empty());
        assert_eq!(shown.activity, activity);
        assert_eq!(shown.retained_failure, expected_failure);
    }
    let shown = show(
        Read::Failed {
            error: &failure,
            previous: Some(&copy),
        },
        "not-present",
    );
    assert!(matches!(shown.body, Body::NoMatches));
    assert_eq!(shown.retained_failure, Some(&failure));
    assert!(matches!(
        show(
            Read::Pending {
                previous: Some(&copy)
            },
            "not-present"
        )
        .body,
        Body::NoMatches
    ));
}

#[test]
fn entries_without_a_fetch_receipt_never_masquerade_as_a_successful_registry() {
    let mut copy = fetched();
    copy.fetched_at = None;
    let failure = Failure::Offline;
    assert!(matches!(
        show(Read::Ready(&copy), "").body,
        Body::NotRequested
    ));
    assert!(matches!(
        show(
            Read::Pending {
                previous: Some(&copy)
            },
            ""
        )
        .body,
        Body::Loading
    ));
    let shown = show(
        Read::Failed {
            error: &failure,
            previous: Some(&copy),
        },
        "",
    );
    assert!(matches!(shown.body, Body::Failed(Failure::Offline)));
    assert!(shown.retained_failure.is_none());
}
