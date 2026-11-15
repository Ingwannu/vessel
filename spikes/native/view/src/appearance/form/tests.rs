//! Real field transitions, including no-op commits and current-saved fallback.
use super::*;
use crate::prefs::{CodeSize, ContentSize, ContentWidth, InterfaceSize};

fn text(value: FieldText) -> String {
    match value {
        FieldText::Value(value) => value.as_str().to_owned(),
        FieldText::WindowWidth => "legacy-full".to_owned(),
    }
}
#[test]
fn size_edits_filter_ascii_digits_while_width_retains_numeric_notation() {
    let edit_size = edit(Field::ContentSize, "a１1.5px");
    assert_eq!(edit_size.text.as_str(), "15");
    assert_eq!(
        edit_size.preview,
        PreviewDelta::Content(Number::parse("15"))
    );
    let edit_width = edit(Field::Width, "1e3");
    assert_eq!(edit_width.text.as_str(), "1e3");
    assert_eq!(edit_width.preview, PreviewDelta::Unchanged);
    let saved = Appearance::default();
    let committed = commit(Field::Width, edit_width.text.as_str(), &saved);
    assert_eq!(text(committed.text), "1000");
    assert_eq!(
        committed.change,
        Some(PreferenceChange::ContentWidth(ContentWidth::Pixels(
            super::super::ContentPixels::try_from(1000).unwrap()
        )))
    );
}
#[test]
fn blank_invalid_and_unchanged_inputs_restore_without_writing() {
    let saved = Appearance::default();
    for (field, raw) in [
        (Field::InterfaceSize, ""),
        (Field::CodeSize, "NaN"),
        (Field::MonoFamily, "bad;family"),
        (Field::Width, "Infinity"),
    ] {
        let result = commit(field, raw, &saved);
        assert_eq!(result.text, saved_text(field, &saved));
        assert_eq!(result.change, None);
    }
    let same = commit(
        Field::Width,
        &super::super::ContentPixels::DEFAULT.get().to_string(),
        &saved,
    );
    assert_eq!(same.change, None);
    assert!(!saved.content_width.resettable());
    let empty_mono = edit(Field::MonoFamily, "");
    assert_eq!(
        empty_mono.preview,
        PreviewDelta::Mono(Some(FontFamily::default()))
    );
    assert_eq!(edit(Field::CodeSize, "").preview, PreviewDelta::Code(None));
}
#[test]
fn commit_clamps_each_axis_and_never_uses_a_stale_saved_fallback() {
    let mut saved = Appearance::default();
    for (field, expected) in [
        (Field::InterfaceSize, InterfaceSize::MAX),
        (Field::ContentSize, ContentSize::MAX),
        (Field::CodeSize, CodeSize::MAX),
    ] {
        assert_eq!(
            text(commit(field, "999", &saved).text),
            expected.to_string()
        );
    }
    saved.mono_family = FontFamily::parse("New External Face").unwrap();
    assert_eq!(
        text(commit(Field::MonoFamily, "bad;face", &saved).text),
        "New External Face"
    );
    assert_eq!(
        commit(Field::MonoFamily, " New External Face ", &saved).change,
        None
    );
    saved.content_width = ContentWidth::Full;
    assert_eq!(
        commit(Field::Width, "", &saved).text,
        FieldText::WindowWidth
    );
    assert_eq!(commit(Field::Width, "", &saved).change, None);
}
