//! Streaming formatter semantics and real TOML output; no file I/O.
use super::*;

#[test]
fn unicode_bytes_comments_and_exact_limit_are_counted_before_append() {
    let source = "# 한글\n";
    let exact = u32::try_from(source.len()).unwrap();
    assert_eq!(
        write_limited(&source, ByteLimit(exact)).unwrap().as_str(),
        source
    );
    assert!(matches!(
        write_limited(&source, ByteLimit(exact.checked_sub(1).unwrap())),
        Err(Error::TooLarge { .. })
    ));
}

struct Fails;
impl fmt::Display for Fails {
    fn fmt(&self, target: &mut fmt::Formatter<'_>) -> fmt::Result {
        target.write_str("partial")?;
        Err(fmt::Error)
    }
}

#[test]
fn a_formatters_own_failure_never_accepts_its_partial_document() {
    assert!(matches!(
        write_limited(&Fails, ByteLimit(100)),
        Err(Error::Format(_))
    ));
}

struct IgnoresFault;
impl fmt::Display for IgnoresFault {
    fn fmt(&self, target: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _rejected = target.write_str("synthetic-private-output");
        let _still_rejected = target.write_str("x");
        Ok(())
    }
}

#[test]
fn a_formatter_cannot_ignore_a_sink_fault_and_publish_a_partial_value() {
    assert!(matches!(
        write_limited(&IgnoresFault, ByteLimit(8)),
        Err(Error::TooLarge { .. })
    ));
}

#[test]
fn safe_errors_never_retain_the_rejected_chunk_in_any_source() {
    let canary = "synthetic-private-output";
    let Err(error) = write_limited(&canary, ByteLimit(8)) else {
        panic!("over limit must refuse")
    };
    let mut source: Option<&dyn std::error::Error> = Some(&error);
    while let Some(error) = source {
        assert!(!error.to_string().contains(canary));
        assert!(!format!("{error:?}").contains(canary));
        source = error.source();
    }
}

#[test]
fn real_toml_display_preserves_comments_and_unicode_within_the_exact_byte_cap() {
    let source = "# keep 우리 주석 😀\ndefault_node = \"work\"\n\n[nodes.work]\nurl = \"ws://machine.example:7077\" # keep endpoint comment\n";
    let document: toml_edit::DocumentMut = source.parse().unwrap();
    assert_eq!(
        document
            .get("default_node")
            .and_then(toml_edit::Item::as_str),
        Some("work")
    );
    let rendered = book(&document).unwrap();
    assert_eq!(rendered.as_str(), source);
    let bytes = u32::try_from(source.len()).unwrap();
    assert_eq!(
        write_limited(&document, ByteLimit(bytes)).unwrap().as_str(),
        source
    );
    assert!(matches!(
        write_limited(&document, ByteLimit(bytes.checked_sub(1).unwrap())),
        Err(Error::TooLarge { .. })
    ));
}

#[test]
fn the_real_book_output_budget_includes_comments_without_truncation() {
    let cap = usize::try_from(MAX_BOOK_OUTPUT_BYTES.0).unwrap();
    let exact = format!("#{}\n", "x".repeat(cap.checked_sub(2).unwrap()));
    let document: toml_edit::DocumentMut = exact.parse().unwrap();
    assert_eq!(book(&document).unwrap().as_str(), exact);
    let over = format!("#{}\n", "x".repeat(cap.checked_sub(1).unwrap()));
    let document: toml_edit::DocumentMut = over.parse().unwrap();
    assert!(matches!(book(&document), Err(Error::TooLarge { .. })));
}
