//! Reference location syntax, Unicode, encoding and navigation-class regressions.

use super::*;

#[test]
fn every_location_spelling_preserves_the_one_based_inclusive_range() {
    for value in [
        "/src/lib.rs:10-20",
        "/src/lib.rs:10:5-20:9",
        "/src/lib.rs(10,5-20,9)",
        "/src/lib.rs lines 10-20",
        "file:///src/lib.rs#L10-L20",
        "/src/lib.rs#L10C2-L20C7",
    ] {
        let target = parse_file(value, Origin::Inferred).unwrap();
        assert_eq!(target.path.as_str(), "/src/lib.rs", "{value}");
        assert_eq!(target.lines.unwrap().start().get(), 10);
        assert_eq!(target.lines.unwrap().end().get(), 20);
    }
    for value in [
        "/src/lib.rs:3:5",
        "/src/lib.rs(3,5)",
        "/src/lib.rs line 3",
        "file:///src/lib.rs#L3C5",
    ] {
        let lines = parse_file(value, Origin::Inferred).unwrap().lines.unwrap();
        assert_eq!((lines.start().get(), lines.end().get()), (3, 3));
    }
}

#[test]
fn href_decode_does_not_change_raw_percent_named_or_unicode_files() {
    assert_eq!(
        parse_file("/tmp/my%20file.md", Origin::Href)
            .unwrap()
            .path
            .as_str(),
        "/tmp/my file.md"
    );
    assert_eq!(
        parse_file("/tmp/my%20file.md", Origin::Inferred)
            .unwrap()
            .path
            .as_str(),
        "/tmp/my%20file.md"
    );
    for value in ["/tmp/한글.rs", "/tmp/%ff.rs", "/tmp/%zz.rs", "/tmp/%.rs"] {
        assert_eq!(
            parse_file(value, Origin::Inferred).unwrap().path.as_str(),
            value
        );
        assert_eq!(
            parse_file(value, Origin::Href).unwrap().path.as_str(),
            value
        );
    }
    assert!(parse_file("/tmp/a%00.md", Origin::Href).is_none());
}

#[test]
fn windows_drive_unc_and_home_paths_keep_their_remote_identity() {
    for (value, expected) in [
        ("C:\\src\\lib.rs:4", "C:/src/lib.rs"),
        ("file:///C:/src/lib.rs#L4", "C:/src/lib.rs"),
        ("\\\\server\\share\\a.md:4", "//server/share/a.md"),
        ("~/src/lib.rs:4", "~/src/lib.rs"),
    ] {
        let target = parse_file(value, Origin::Inferred).unwrap();
        assert_eq!(target.path.as_str(), expected, "{value}");
        assert_eq!(target.lines.unwrap().start().get(), 4);
        assert!(matches!(
            classify(value, Origin::Inferred, Workspace::Unknown),
            Some(Link::File(_))
        ));
    }
    // Host-bearing file URLs must not silently read a different host's local path.
    assert!(parse_file("file://another-host/share/a.md", Origin::Href).is_none());
}

#[test]
fn inferred_relative_files_require_workspace_lookup_but_explicit_hrefs_do_not() {
    assert!(matches!(
        classify("src/lib.rs:4", Origin::Inferred, Workspace::Known),
        Some(Link::Lookup(_))
    ));
    assert!(classify("src/lib.rs", Origin::Inferred, Workspace::Unknown).is_none());
    assert!(classify("just-words", Origin::Inferred, Workspace::Known).is_none());
    assert!(matches!(
        classify("README", Origin::Href, Workspace::Known),
        Some(Link::File(_))
    ));
    assert!(matches!(
        classify(
            "https://example.com/a.rs:4",
            Origin::Href,
            Workspace::Unknown
        ),
        Some(Link::External(_))
    ));
}

#[test]
fn arbitrary_schemes_controls_and_reversed_or_zero_fragment_ranges_do_not_open() {
    for value in [
        "javascript:alert(1)",
        "data:text/plain,hello",
        "ssh://host/file.rs",
        "file:///a.rs#L0",
        "/a.rs#L20-L10",
        "/a.rs#L3-L0",
        "/a.rs#L4294967296",
        "a\u{0}b.rs",
        "",
    ] {
        assert!(
            classify(value, Origin::Href, Workspace::Known).is_none(),
            "{value}"
        );
    }
}

#[test]
fn fragment_grammar_requires_digits_and_at_most_one_line_or_column_marker() {
    for value in [
        "/a.rs#LL3",
        "/a.rs#L3Cbad",
        "/a.rs#L+3",
        "/a.rs#L3-L+4",
        "/a.rs#L3C4C5",
        "/a.rs#L3C",
        "/a.rs#L3-LL4",
    ] {
        assert!(parse_file(value, Origin::Href).is_none(), "{value}");
    }
}

#[test]
fn file_tokens_use_the_reference_whitespace_set_without_erasing_nel_controls() {
    for origin in [Origin::Href, Origin::Inferred] {
        let target = parse_file("\u{feff} /src/lib.rs:4 \u{feff}", origin).unwrap();
        assert_eq!(target.path.as_str(), "/src/lib.rs");
        assert_eq!(target.lines.unwrap().start().get(), 4);
        for value in ["\u{0085}/src/lib.rs", "/src/lib.rs\u{0085}"] {
            assert!(classify(value, origin, Workspace::Known).is_none());
        }
    }
    assert!(matches!(
        classify(
            "\u{feff}HTTPS://example.com/a",
            Origin::Href,
            Workspace::Unknown
        ),
        Some(Link::External(_))
    ));
}
