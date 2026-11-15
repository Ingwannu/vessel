//! Wire values stay bounded and canonical before any read or view work.
use super::*;

#[test]
fn whitespace_unicode_and_chunked_previews_follow_one_rule() {
    assert_eq!(
        PromptPreview::from_text("  한글\n prompt \t").as_str(),
        "한글 prompt"
    );
    let mut chunks = PreviewBuilder::default();
    for text in ["  한", "글\n", "prompt "] {
        chunks.push(text);
    }
    assert_eq!(
        chunks.finish(),
        PromptPreview::from_text("  한글\n prompt ")
    );
    assert_eq!(
        PromptPreview::from_text("\u{feff}a\u{2003}b").as_str(),
        "a b"
    );
    let exact = "a".repeat(PREVIEW_UNITS);
    assert_eq!(PromptPreview::from_text(&exact).as_str(), exact);
    let cut = PromptPreview::from_text(&"한".repeat(PREVIEW_UNITS + 1));
    assert_eq!(cut.as_str().encode_utf16().count(), PREVIEW_UNITS);
    assert!(cut.as_str().ends_with('…'));
    let emoji = PromptPreview::from_text(&"😀".repeat(PREVIEW_UNITS));
    assert!(emoji.as_str().encode_utf16().count() <= PREVIEW_UNITS);
    assert!(emoji.as_str().ends_with('…'));
    assert!(serde_json::from_str::<PromptPreview>(r#"" unnormalized  text ""#).is_err());
}

#[test]
fn limits_and_order_are_validated_on_constructor_and_wire_decode() {
    assert!(PromptIndexLimit::new(0).is_err());
    assert!(PromptIndexLimit::new(PAGE_ENTRIES.saturating_add(1)).is_err());
    assert!(PromptIndexLimit::new(PAGE_ENTRIES).is_ok());
    let session = SessionId::random();
    let epoch = PromptIndexEpoch::new(EventSeq::new(0), None).unwrap();
    let row = |seq| PromptLocation {
        seq: EventSeq::new(seq),
        timestamp: "2026-01-01T00:00:00Z".parse().unwrap(),
        preview: PromptPreview::from_text("prompt"),
    };
    assert!(PromptIndexPage::new(session, epoch, vec![row(2), row(1)], None).is_err());
    assert!(PromptIndexPage::new(session, epoch, vec![row(1), row(1)], None).is_err());
    assert!(PromptIndexPage::new(session, epoch, vec![row(1)], Some(EventSeq::new(2))).is_err());
    let page = PromptIndexPage::new(session, epoch, vec![row(1)], Some(EventSeq::new(1))).unwrap();
    assert_eq!(
        serde_json::from_str::<PromptIndexPage>(&serde_json::to_string(&page).unwrap()).unwrap(),
        page
    );
    assert!(
        PromptIndexPage::new(
            session,
            epoch,
            (0..=u64::from(PAGE_ENTRIES)).map(row).collect(),
            None
        )
        .is_err()
    );
}
