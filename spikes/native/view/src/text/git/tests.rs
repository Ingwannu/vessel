//! Whole sentences, including both languages and exact diagnostic data.
//! Existing git refusals are the workspace table's; these cases own the
//! new preparation errors and the menu's branch/count sentences.

use vessel_proto::workspace::{InvalidText, TextKind, TextProblem};

use crate::workspace::git::SwitchProblem;
use crate::{GitFailureText, Lang};

fn assert_words(actual: &GitFailureText, expected: (&str, &str, Option<&str>)) {
    assert_eq!(
        (
            actual.title.as_str(),
            actual.next.as_str(),
            actual.detail.as_deref()
        ),
        expected
    );
}

#[test]
fn empty_and_oversized_commit_messages_say_what_to_fix_in_both_languages() {
    let empty = InvalidText {
        kind: TextKind::CommitMessage,
        problem: TextProblem::Empty,
    };
    assert_words(
        &Lang::En.git_message_failure(&empty),
        (
            "The commit message is empty.",
            "Write a message before committing.",
            None,
        ),
    );
    assert_words(
        &Lang::Ko.git_message_failure(&empty),
        (
            "커밋 메시지가 비어 있습니다.",
            "커밋하기 전에 메시지를 쓰세요.",
            None,
        ),
    );
    let long = InvalidText {
        kind: TextKind::CommitMessage,
        problem: TextProblem::TooLong { max: 65_536 },
    };
    assert_words(
        &Lang::En.git_message_failure(&long),
        (
            "The commit message is too long.",
            "Shorten the message and try again.",
            Some("Maximum length: 64 KB."),
        ),
    );
    assert_words(
        &Lang::Ko.git_message_failure(&long),
        (
            "커밋 메시지가 너무 깁니다.",
            "메시지를 줄인 뒤 다시 시도하세요.",
            Some("최대 길이: 64 KB."),
        ),
    );
}

#[test]
fn a_small_name_limit_is_written_as_bytes_in_each_language() {
    let error = InvalidText {
        kind: TextKind::Branch,
        problem: TextProblem::TooLong { max: 255 },
    };
    assert_words(
        &Lang::En.git_name_failure(&error),
        (
            "The branch name is too long.",
            "Choose a shorter branch name.",
            Some("Maximum length: 255 bytes."),
        ),
    );
    assert_words(
        &Lang::Ko.git_name_failure(&error),
        (
            "브랜치 이름이 너무 깁니다.",
            "더 짧은 브랜치 이름을 고르세요.",
            Some("최대 길이: 255바이트."),
        ),
    );
}

#[test]
fn character_and_shape_failures_keep_the_exact_detail_below_localized_guidance() {
    let nul = InvalidText {
        kind: TextKind::CommitMessage,
        problem: TextProblem::Character { character: '\0' },
    };
    assert_words(
        &Lang::En.git_message_failure(&nul),
        (
            "The commit message contains a character Git cannot use.",
            "Remove that character and try again.",
            Some("Unsupported character: NUL."),
        ),
    );
    assert_words(
        &Lang::Ko.git_message_failure(&nul),
        (
            "커밋 메시지에 Git이 쓸 수 없는 문자가 있습니다.",
            "그 문자를 지운 뒤 다시 시도하세요.",
            Some("쓸 수 없는 문자: NUL."),
        ),
    );
    let shape = InvalidText {
        kind: TextKind::CommitMessage,
        problem: TextProblem::Shape {
            rule: "an unfamiliar parser rule",
        },
    };
    assert_words(
        &Lang::En.git_message_failure(&shape),
        (
            "Git cannot use this commit message.",
            "Write a different message and try again.",
            Some("an unfamiliar parser rule"),
        ),
    );
    assert_words(
        &Lang::Ko.git_message_failure(&shape),
        (
            "Git이 이 커밋 메시지를 쓸 수 없습니다.",
            "다른 메시지를 쓰고 다시 시도하세요.",
            Some("an unfamiliar parser rule"),
        ),
    );
}

type Sentences = (&'static str, &'static str);

const BRANCH_CASES: [(TextProblem, Sentences, Sentences); 4] = [
    (
        TextProblem::Empty,
        (
            "The branch name is empty.",
            "Enter a branch name before continuing.",
        ),
        (
            "브랜치 이름이 비어 있습니다.",
            "계속하려면 브랜치 이름을 입력하세요.",
        ),
    ),
    (
        TextProblem::TooLong { max: 255 },
        (
            "The branch name is too long.",
            "Choose a shorter branch name.",
        ),
        (
            "브랜치 이름이 너무 깁니다.",
            "더 짧은 브랜치 이름을 고르세요.",
        ),
    ),
    (
        TextProblem::Character { character: ' ' },
        (
            "The branch name contains a character Git cannot use.",
            "Remove whitespace or reserved punctuation and try again.",
        ),
        (
            "브랜치 이름에 Git이 쓸 수 없는 문자가 있습니다.",
            "공백이나 예약된 문장 부호를 지운 뒤 다시 시도하세요.",
        ),
    ),
    (
        TextProblem::Shape {
            rule: "reserved name",
        },
        (
            "Git cannot use this branch name.",
            "Choose another branch name.",
        ),
        (
            "Git이 이 브랜치 이름을 쓸 수 없습니다.",
            "다른 브랜치 이름을 고르세요.",
        ),
    ),
];

#[test]
fn branch_failures_are_localized_for_every_wire_constructor_reason() {
    for (problem, en, ko) in BRANCH_CASES {
        let error = InvalidText {
            kind: TextKind::Branch,
            problem,
        };
        for (lang, (title, next)) in [(Lang::En, en), (Lang::Ko, ko)] {
            let text = lang.git_name_failure(&error);
            assert_eq!((text.title.as_str(), text.next.as_str()), (title, next));
            let switched = lang.git_switch_failure(&SwitchProblem::InvalidName(error.clone()));
            assert_eq!(switched, text);
        }
    }
}

#[test]
fn mismatched_remote_and_exhausted_keys_never_show_a_display_message_as_guidance() {
    let mismatch = SwitchProblem::RemoteMismatch {
        reference: "other/feature".parse().unwrap(),
        remote: "origin".parse().unwrap(),
    };
    assert_words(
        &Lang::En.git_switch_failure(&mismatch),
        (
            "The listed branch does not match its remote.",
            "Refresh the branch list and choose the branch again.",
            Some("Listed branch: other/feature. Remote: origin."),
        ),
    );
    assert_words(
        &Lang::Ko.git_switch_failure(&mismatch),
        (
            "목록의 브랜치가 원격 저장소와 맞지 않습니다.",
            "브랜치 목록을 새로 고친 뒤 브랜치를 다시 고르세요.",
            Some("목록의 브랜치: other/feature. 원격 저장소: origin."),
        ),
    );
    assert_words(
        &Lang::En.git_keys_exhausted(),
        (
            "An app limit prevents another git action.",
            "Quit and restart Vessel, then try again.",
            None,
        ),
    );
    assert_words(
        &Lang::Ko.git_keys_exhausted(),
        (
            "앱의 한도 때문에 Git 동작을 더 실행할 수 없습니다.",
            "Vessel을 완전히 종료하고 다시 실행한 뒤 시도하세요.",
            None,
        ),
    );
}

#[test]
fn character_details_use_names_for_invisible_characters_and_preserve_other_values() {
    for (character, en, ko) in [
        (
            ' ',
            "Unsupported character: space.",
            "쓸 수 없는 문자: 공백.",
        ),
        (
            '\n',
            "Unsupported character: line break.",
            "쓸 수 없는 문자: 줄바꿈.",
        ),
        ('\t', "Unsupported character: tab.", "쓸 수 없는 문자: 탭."),
        (
            '\u{1f}',
            "Unsupported character: U+001F.",
            "쓸 수 없는 문자: U+001F.",
        ),
        ('a', "Unsupported character: ‘a’.", "쓸 수 없는 문자: ‘a’."),
    ] {
        let error = InvalidText {
            kind: TextKind::Branch,
            problem: TextProblem::Character { character },
        };
        assert_eq!(
            Lang::En.git_name_failure(&error).detail.as_deref(),
            Some(en)
        );
        assert_eq!(
            Lang::Ko.git_name_failure(&error).detail.as_deref(),
            Some(ko)
        );
    }
}
