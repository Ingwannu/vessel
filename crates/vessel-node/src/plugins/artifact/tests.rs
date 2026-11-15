use super::*;

fn file(path: &str, bytes: u64, executable: bool) -> Record {
    Record::new(
        path,
        Kind::File {
            bytes,
            executable,
            hash: ContentHash::from_sha256([1; 32]),
        },
    )
    .expect("record")
}
#[test]
fn digest_is_order_independent_and_distinguishes_kind_content_and_execute_bit() {
    let left = TreeDigest::new(vec![file("a", 1, false), file("b", 2, true)]).expect("digest");
    let right = TreeDigest::new(vec![file("b", 2, true), file("a", 1, false)]).expect("digest");
    assert_eq!(left, right);
    assert_ne!(
        left,
        TreeDigest::new(vec![file("a", 1, true), file("b", 2, true)]).expect("changed execute")
    );
    let different = Record::new(
        "a",
        Kind::File {
            bytes: 1,
            executable: false,
            hash: ContentHash::from_sha256([2; 32]),
        },
    )
    .expect("record");
    assert_ne!(
        left,
        TreeDigest::new(vec![different, file("b", 2, true)]).expect("changed content")
    );
    let link = Record::new(
        "a",
        Kind::Link {
            target: LinkTarget::new("b").expect("bounded spelling only"),
        },
    )
    .expect("record");
    assert_ne!(
        left,
        TreeDigest::new(vec![link, file("b", 2, true)]).expect("different kind")
    );
    let directory = Record::new("a", Kind::Directory { executable: true }).expect("directory fact");
    assert_ne!(
        left,
        TreeDigest::new(vec![directory, file("b", 2, true)]).expect("directory kind")
    );
    assert!(matches!(
        TreeDigest::new(vec![file("same", 0, false), file("same", 0, false)]),
        Err(Problem::Duplicate)
    ));
}
#[test]
fn all_budget_refusals_preserve_admitted_facts_and_include_directory_and_link_counts() {
    let mut budget = Budget::default();
    budget
        .admit(&file("first", MAX_FILE_BYTES, false))
        .expect("first");
    budget
        .admit(&file("second", MAX_FILE_BYTES, false))
        .expect("total cap");
    let before = budget;
    assert!(matches!(
        budget.admit(&file("third", 1, false)),
        Err(Problem::Capacity(Limit::TotalBytes))
    ));
    assert_eq!(budget, before);
    assert!(matches!(
        budget.admit(&file("large", MAX_FILE_BYTES.saturating_add(1), false)),
        Err(Problem::Capacity(Limit::FileBytes))
    ));
    assert_eq!(budget, before);
    let deep = (0..MAX_DEPTH.saturating_add(1))
        .map(|_| "d")
        .collect::<Vec<_>>()
        .join("/");
    assert!(matches!(
        budget.admit(&file(&deep, 0, false)),
        Err(Problem::Capacity(Limit::Depth))
    ));
    assert_eq!(budget, before);
    let mut entries = Budget::default();
    let directory =
        Record::new("directory", Kind::Directory { executable: true }).expect("directory");
    let link = Record::new(
        "link",
        Kind::Link {
            target: LinkTarget::new("directory").expect("spelling"),
        },
    )
    .expect("link");
    for _ in 0..MAX_ENTRIES {
        entries.admit(&directory).expect("bounded count");
    }
    let before = entries;
    assert!(matches!(
        entries.admit(&link),
        Err(Problem::Capacity(Limit::Entries))
    ));
    assert_eq!(entries, before);
}
#[test]
fn names_and_links_are_bounded_and_invalid_name_diagnostics_keep_input_sealed() {
    assert!(matches!(
        Record::new(
            &"x".repeat(MAX_NAME_BYTES.saturating_add(1)),
            Kind::Directory { executable: true }
        ),
        Err(Problem::Capacity(Limit::NameBytes))
    ));
    assert!(matches!(
        LinkTarget::new(&"x".repeat(MAX_NAME_BYTES.saturating_add(1))),
        Err(Problem::Capacity(Limit::NameBytes))
    ));
    assert!(matches!(LinkTarget::new(""), Err(Problem::Link)));
    assert!(matches!(LinkTarget::new("bad\0target"), Err(Problem::Link)));
    let error = match Record::new(
        "../PRIVATE-ARTIFACT-CANARY",
        Kind::Directory { executable: true },
    ) {
        Err(error) => error,
        Ok(_) => panic!("invalid path accepted"),
    };
    let mut source: Option<&(dyn Error + 'static)> = Some(&error);
    while let Some(error) = source {
        assert!(!format!("{error:?} {error}").contains("PRIVATE-ARTIFACT-CANARY"));
        source = error.source();
    }
}
