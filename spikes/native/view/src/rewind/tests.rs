//! Exact menu and notice wording in both languages, including retranslating
//! an existing outcome rather than issuing another rewind.
use super::*;
use crate::localization::{ENGLISH, Locale};

#[test]
fn the_menu_and_copy_labels_keep_the_existing_sentences() {
    let en = Translator::get(ENGLISH).unwrap().rewind();
    let ko = Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .rewind();
    assert_eq!(en.tooltip(), "Rewind to this message");
    assert_eq!(ko.tooltip(), "이 메시지로 되감기");
    assert_eq!(en.warning(), "This action cannot be undone");
    assert_eq!(ko.warning(), "이 작업은 되돌릴 수 없습니다");
    assert_eq!(en.conversation(), "Rewind conversation");
    assert_eq!(ko.conversation(), "대화 되감기");
    assert_eq!(en.copy_message(), "Copy message");
    assert_eq!(ko.copy_message(), "메시지 복사");
}

#[test]
fn a_notice_keeps_its_complete_sentence_and_rewords_its_retained_input() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    for (message, english, korean) in [
        ("", "Failed to rewind agent", "에이전트를 되감지 못했습니다"),
        (
            " ",
            "Failed to rewind agent",
            "에이전트를 되감지 못했습니다",
        ),
        (
            "node refusal",
            "Failed to rewind agent: \u{2068}node refusal\u{2069}",
            "에이전트를 되감지 못했습니다: \u{2068}node refusal\u{2069}",
        ),
    ] {
        let mut notice = FailureNotice::new(message, en).unwrap();
        assert_eq!(&*notice.words(), english);
        notice.translate(ko).unwrap();
        assert_eq!(&*notice.words(), korean);
    }
}
