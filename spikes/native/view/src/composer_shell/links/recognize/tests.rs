//! Reference complete inline-code classification and conservative prose token boundaries.

use super::*;
use crate::composer_shell::links::Link;

#[test]
fn inline_code_keeps_valid_reference_paths_and_excludes_invalid_locations_or_domains() {
    for value in [
        "src/app/[id].tsx",
        "src/app/(tabs)/index.tsx",
        "src/a$b.ts",
        ".gitignore",
        "my%20file.ts",
        "src/index.ts#L10-L20",
        "src/index.ts:10-20",
        "README.md",
        "./scripts/run",
        "src/file.ts(10,2)",
        "src/file.ts lines 10-20",
        "/tmp/[route].tsx",
        "file:///tmp/file.ts#L10-L20",
        "C:\\repo\\file.ts",
        "docs/说明.md",
        "https://example.com",
    ] {
        assert_eq!(
            tokens(value, Kind::Code),
            [Token {
                range: 0..value.len()
            }],
            "{value}"
        );
    }
    for value in [
        "src/index.ts:20-10",
        "src/index.ts#L0",
        "src/index.ts?raw",
        "before/after",
        "word.word",
        "example.com/src/lib.rs",
        "/tmp/file.rs:0",
        "C:relative.rs",
    ] {
        assert!(tokens(value, Kind::Code).is_empty(), "{value}");
    }
}

#[test]
fn prose_ranges_preserve_unicode_raw_percent_names_and_sentence_punctuation() {
    let text =
        "See src/lib.rs:4, docs/说明.md and my%20file.ts. Nothing about example.com/src/lib.rs.";
    let found: Vec<_> = tokens(text, Kind::Text)
        .iter()
        .map(|token| text.get(token.range.clone()).unwrap())
        .collect();
    assert_eq!(
        found,
        [
            "src/lib.rs:4",
            "docs/说明.md",
            "my%20file.ts",
            "example.com/src/lib.rs"
        ]
    );
    assert!(tokens("not [id].tsx or .gitignore or just-words", Kind::Text).is_empty());
}

#[test]
fn prose_grammar_matches_reference_prefix_and_location_boundaries_before_resolution() {
    for value in [
        "/tmp/file",
        "./scripts/run",
        "src/bin/tool",
        "src/lib.rs:4",
        "README.md",
        "https://localhost/a.rs",
        "docs/说明.md",
        "/srv/repo/secret",
        "C:\\repo\\file.ts",
        "a.rs:0",
        "a.rs:20-10",
    ] {
        assert_eq!(
            tokens(value, Kind::Text),
            [Token {
                range: 0..value.len()
            }],
            "{value}"
        );
    }
    for value in [
        "/src",
        "~/file.rs",
        ".gitignore",
        "not [id].tsx",
        "word.word",
    ] {
        assert!(tokens(value, Kind::Text).is_empty(), "{value}");
    }
}

#[test]
fn encoded_href_delimiters_stay_filename_bytes_and_http_classification_is_typed() {
    let target = super::super::parse_file("src/a%3Ab.rs", Origin::Href).unwrap();
    assert_eq!(target.path.as_str(), "src/a:b.rs");
    assert!(super::super::parse_file("/tmp/a.rs?raw", Origin::Href).is_none());
    assert!(matches!(
        classify("HTTPS://example.com", Origin::Href, Workspace::Unknown),
        Some(Link::External(_))
    ));
}
