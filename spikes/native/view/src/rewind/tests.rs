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

const RECOVERIES: &[(Problem, &str, &str)] = &[
    (
        Problem::Point,
        "The selected point cannot be used to rewind this conversation. Choose another message and try again.",
        "고른 지점으로 이 대화를 되감을 수 없습니다. 다른 메시지를 골라 다시 시도하세요.",
    ),
    (
        Problem::Access,
        "This device is not allowed to rewind this conversation.",
        "이 기기에는 이 대화를 되감을 권한이 없습니다.",
    ),
    (
        Problem::Version,
        "Update Vessel on this device and the machine before rewinding.",
        "되감기 전에 이 기기와 머신의 Vessel을 업데이트하세요.",
    ),
    (
        Problem::Missing,
        "This conversation is no longer available.",
        "이 대화를 더 이상 사용할 수 없습니다.",
    ),
    (
        Problem::NotReady,
        "This conversation cannot be rewound right now. Open it again before retrying.",
        "지금은 이 대화를 되감을 수 없습니다. 다시 열고 시도하세요.",
    ),
    (
        Problem::Unsupported,
        "This agent cannot rewind conversations.",
        "이 에이전트는 대화를 되감을 수 없습니다.",
    ),
    (
        Problem::AgentMissing,
        "This conversation's agent is not installed on the machine.",
        "이 대화의 에이전트가 머신에 설치되어 있지 않습니다.",
    ),
    (
        Problem::Start,
        "The agent could not be started to rewind this conversation. Try again.",
        "대화를 되감을 에이전트를 시작하지 못했습니다. 다시 시도하세요.",
    ),
    (
        Problem::Unavailable,
        "The machine cannot rewind this conversation right now. Try again later.",
        "머신이 지금 이 대화를 되감을 수 없습니다. 나중에 다시 시도하세요.",
    ),
    (
        Problem::Failed,
        "Could not confirm the rewind. Open the conversation again before retrying.",
        "되감기 결과를 확인하지 못했습니다. 대화를 다시 열고 시도하세요.",
    ),
];

#[test]
fn every_refusal_is_a_whole_sentence_and_rewords_the_same_typed_problem() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    for &(problem, english, korean) in RECOVERIES {
        let mut notice = FailureNotice::new(problem, en);
        assert_eq!(&*notice.words(), english);
        notice.translate(ko);
        assert_eq!(&*notice.words(), korean);
    }
}

#[test]
fn all_node_codes_choose_explicit_recovery_kinds() {
    use vessel_proto::ErrorCode;
    for (code, expected) in [
        (ErrorCode::InvalidRequest, Problem::Point),
        (ErrorCode::Unauthorized, Problem::Access),
        (ErrorCode::Forbidden, Problem::Access),
        (ErrorCode::ProtocolMismatch, Problem::Version),
        (ErrorCode::NotFound, Problem::Missing),
        (ErrorCode::Conflict, Problem::NotReady),
        (ErrorCode::Unsupported, Problem::Unsupported),
        (ErrorCode::HarnessNotInstalled, Problem::AgentMissing),
        (ErrorCode::SpawnFailed, Problem::Start),
        (ErrorCode::Unavailable, Problem::Unavailable),
        (ErrorCode::Internal, Problem::Failed),
    ] {
        assert_eq!(Problem::from(code), expected);
    }
}
