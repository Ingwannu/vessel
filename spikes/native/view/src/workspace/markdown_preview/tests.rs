use super::*;
fn source(text: &str) -> MarkdownSource {
    Arc::from(text).try_into().unwrap()
}

#[test]
fn metadata_preserves_order_and_formats_scalars_and_lists() {
    let document = parse(&source(
        "\u{feff}---\r\ntitle: Hello\r\ncount: 2\r\nenabled: true\r\nempty: null\r\ntags: [one, 2, false]\r\nnested:\r\n  key: value\r\n---\r\n# Body\r\n",
    ));
    assert_eq!(
        document
            .front_matter
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["title", "count", "enabled", "empty", "tags", "nested"]
    );
    assert_eq!(
        document
            .front_matter
            .iter()
            .take(5)
            .map(|row| row.value.as_str())
            .collect::<Vec<_>>(),
        ["Hello", "2", "true", "null", "one, 2, false"]
    );
    assert!(
        document
            .front_matter
            .last()
            .unwrap()
            .value
            .contains("key: value")
    );
    assert_eq!(&*document.body, "# Body\r\n");
}

#[test]
fn invalid_nonmapping_duplicate_and_deep_headers_keep_exact_original_source() {
    let deep = format!(
        "---\nx: {}0{}\n---\nbody",
        "[".repeat(MAX_FRONT_MATTER_DEPTH + 1),
        "]".repeat(MAX_FRONT_MATTER_DEPTH + 1)
    );
    let long = format!("---\nx: {}\n---\nbody", "a".repeat(MAX_FRONT_MATTER_BYTES));
    for original in [
        "normal\n---\nsource",
        "---\n[a, b]\n---\nbody",
        "---\nx: [broken\n---\nbody",
        "---\nx: 1\nx: 2\n---\nbody",
        deep.as_str(),
        long.as_str(),
    ] {
        let source = source(original);
        let document = parse(&source);
        assert!(document.front_matter.is_empty(), "{original}");
        assert!(Arc::ptr_eq(&document.body, &source.0));
        assert_eq!(&*document.body, original);
    }
}

#[test]
fn aliases_work_within_budget_but_amplification_falls_back_to_source() {
    let document = parse(&source("---\nfirst: &name hello\nsecond: *name\n---\nbody"));
    assert_eq!(document.front_matter.len(), 2);
    assert_eq!(document.front_matter.last().unwrap().value, "hello");
    let original = format!(
        "---\nx: &x hello\ny: [{}]\n---\nbody",
        vec!["*x"; MAX_FRONT_MATTER_ALIASES + 1].join(", ")
    );
    let document = parse(&source(&original));
    assert_eq!(&*document.body, original);
    assert!(document.front_matter.is_empty());
}

#[test]
fn property_like_text_is_literal_and_source_budget_is_checked_before_parsing() {
    let document = parse(&source("---\nvalue: '${HOME}'\n---\nbody"));
    assert_eq!(document.front_matter.first().unwrap().value, "${HOME}");
    let too_large: Arc<str> = Arc::from("x".repeat(MAX_MARKDOWN_BYTES + 1));
    assert!(MarkdownSource::try_from(too_large).is_err());
}
