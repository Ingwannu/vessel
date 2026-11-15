use super::compare;

fn sorted<const N: usize>(mut names: [&str; N]) -> [&str; N] {
    names.sort_by(|one, other| compare(one, other));
    names
}

#[test]
fn the_provider_catalogs_original_six_titles_keep_the_reference_order() {
    // Paseo components/provider-catalog-search.test.ts, empty query fixture.
    assert_eq!(
        sorted([
            "Kiro CLI",
            "goose",
            "Kimi Code CLI",
            "Gemini CLI",
            "Qwen Code",
            "Codex",
        ]),
        [
            "Codex",
            "Gemini CLI",
            "goose",
            "Kimi Code CLI",
            "Kiro CLI",
            "Qwen Code",
        ]
    );
}

#[test]
fn case_ties_use_the_complete_original_string_and_equal_names_are_equal() {
    assert_eq!(
        sorted(["alpha", "ALPHA", "Alpha"]),
        ["ALPHA", "Alpha", "alpha"]
    );
    assert_eq!(
        compare("동일한 이름", "동일한 이름"),
        std::cmp::Ordering::Equal
    );
    assert_eq!(sorted(["ab", "a", "A", ""]), ["", "A", "a", "ab"]);
}

#[test]
fn unicode_lowercase_expansions_and_context_are_not_ascii_or_per_char_folds() {
    assert_eq!(
        sorted(["İ", "i\u{307}", "I", "i"]),
        ["I", "i", "i\u{307}", "İ"]
    );
    assert_eq!(sorted(["ΟΣ", "οσ", "Ος", "ος"]), ["ΟΣ", "Ος", "ος", "οσ"]);
}

#[test]
fn korean_accents_and_canonical_forms_have_explicit_scalar_order() {
    assert_eq!(sorted(["한글", "나무", "가방"]), ["가방", "나무", "한글"]);
    assert_eq!(
        sorted(["é", "z", "e\u{301}", "e"]),
        ["e", "e\u{301}", "z", "é"]
    );
}

#[test]
fn supplementary_characters_use_scalar_order_instead_of_utf16_units() {
    assert_eq!(
        sorted(["\u{1f600}", "\u{10428}", "\u{e000}", "\u{10400}"]),
        ["\u{e000}", "\u{10400}", "\u{10428}", "\u{1f600}"]
    );
}
