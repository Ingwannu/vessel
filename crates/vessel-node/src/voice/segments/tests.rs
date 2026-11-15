use super::*;

fn parts(text: &str) -> Vec<String> {
    split(&SpeechText::try_from(text.to_owned()).unwrap())
        .unwrap()
        .into_iter()
        .map(|text| text.as_str().to_owned())
        .collect()
}

#[test]
fn sentences_and_javascript_whitespace_follow_the_reference() {
    assert_eq!(
        parts("\u{feff}One.\nTwo!\tThree?\u{2028}Four"),
        ["One.", "Two!", "Three?", "Four"]
    );
    assert_eq!(parts("a\u{85}b"), ["a\u{85}b"]);
    assert_eq!(parts("Dr. Smith"), ["Dr.", "Smith"]);
}

#[test]
fn clauses_are_greedily_packed_then_words_and_scalars_are_bounded() {
    let a = "a".repeat(125);
    let b = "b".repeat(125);
    let c = "c".repeat(125);
    assert_eq!(parts(&format!("{a}, {b}; {c}")), [format!("{a}, {b};"), c]);
    let text = "word ".repeat(200);
    let result = parts(&text);
    assert!(result.iter().all(|part| units(part) <= MAX_SEGMENT_UNITS));
    assert_eq!(result.join(" "), text.trim());
    assert_eq!(
        parts(&"x".repeat(521))
            .iter()
            .map(String::len)
            .collect::<Vec<_>>(),
        [260, 260, 1]
    );
}

#[test]
fn emoji_hard_cut_never_splits_a_surrogate_pair() {
    let text = format!("{}😀😀", "x".repeat(259));
    let result = parts(&text);
    assert_eq!(result, ["x".repeat(259), "😀😀".to_owned()]);
    assert!(result.iter().all(|part| units(part) <= MAX_SEGMENT_UNITS));
    assert_eq!(result.concat(), text);
}
