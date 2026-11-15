//! Whole sentences and the typed picker's labels retain their old wording.
use crate::localization::{Locale, Translator};
use crate::prefs::ConversationDisplay;

#[test]
fn chat_sentences_and_display_choices_keep_the_existing_words() {
    let en = Translator::get(Locale::from_tag("en").unwrap())
        .unwrap()
        .chat();
    let ko = Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .chat();
    assert_eq!(en.title(), "Chat");
    assert_eq!(ko.title(), "채팅");
    assert_eq!(en.detail_level(), "Detail level");
    assert_eq!(ko.detail_level(), "세부 수준");
    assert_eq!(en.display(), "Chat display");
    assert_eq!(ko.display(), "채팅 표시");
    assert_eq!(
        en.description(),
        "How much of an agent's work the chat shows"
    );
    assert_eq!(ko.description(), "채팅에 표시할 에이전트 작업의 양");
    assert_eq!(en.display_choice(ConversationDisplay::Full), "Full");
    assert_eq!(ko.display_choice(ConversationDisplay::Full), "전체");
    assert_eq!(
        en.display_choice(ConversationDisplay::CollapseTools),
        "Collapse tool calls"
    );
    assert_eq!(
        ko.display_choice(ConversationDisplay::CollapseTools),
        "도구 호출 접기"
    );
}

#[test]
fn every_display_choice_has_its_whole_english_and_korean_label_in_source_order() {
    let en = Translator::get(Locale::from_tag("en").unwrap())
        .unwrap()
        .chat();
    let ko = Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .chat();
    let actual: Vec<_> = ConversationDisplay::ALL
        .into_iter()
        .map(|display| (en.display_choice(display), ko.display_choice(display)))
        .collect();
    assert_eq!(
        actual,
        [
            ("Full", "전체"),
            ("Collapse tool calls", "도구 호출 접기"),
            ("Collapse turns", "턴 접기")
        ]
    );
}
