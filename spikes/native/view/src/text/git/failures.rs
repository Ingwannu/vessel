//! Local preparation failures. The two complete tables make adding a
//! reason in only one language a compile error, as in the other areas.

use vessel_proto::workspace::{InvalidText, TextProblem};

use crate::Lang;
use crate::workspace::git::SwitchProblem;

/// A failure explained beside the form that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFailureText {
    /// What happened, in the reader's language.
    pub title: String,
    /// What the person should do next, in the reader's language.
    pub next: String,
    /// Exact diagnostic data, beneath the explanation rather than in
    /// place of it. A known character or limit is localized too.
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct Lines {
    title: &'static str,
    next: &'static str,
}

const fn lines(title: &'static str, next: &'static str) -> Lines {
    Lines { title, next }
}

#[derive(Debug)]
struct Words {
    message_empty: Lines,
    message_long: Lines,
    message_character: Lines,
    message_shape: Lines,
    name_empty: Lines,
    name_long: Lines,
    name_character: Lines,
    name_shape: Lines,
    remote_mismatch: Lines,
    keys_exhausted: Lines,
}

const EN: Words = Words {
    message_empty: lines(
        "The commit message is empty.",
        "Write a message before committing.",
    ),
    message_long: lines(
        "The commit message is too long.",
        "Shorten the message and try again.",
    ),
    message_character: lines(
        "The commit message contains a character Git cannot use.",
        "Remove that character and try again.",
    ),
    message_shape: lines(
        "Git cannot use this commit message.",
        "Write a different message and try again.",
    ),
    name_empty: lines(
        "The branch name is empty.",
        "Enter a branch name before continuing.",
    ),
    name_long: lines(
        "The branch name is too long.",
        "Choose a shorter branch name.",
    ),
    name_character: lines(
        "The branch name contains a character Git cannot use.",
        "Remove whitespace or reserved punctuation and try again.",
    ),
    name_shape: lines(
        "Git cannot use this branch name.",
        "Choose another branch name.",
    ),
    remote_mismatch: lines(
        "The listed branch does not match its remote.",
        "Refresh the branch list and choose the branch again.",
    ),
    keys_exhausted: lines(
        "An app limit prevents another git action.",
        "Quit and restart Vessel, then try again.",
    ),
};

const KO: Words = Words {
    message_empty: lines(
        "커밋 메시지가 비어 있습니다.",
        "커밋하기 전에 메시지를 쓰세요.",
    ),
    message_long: lines(
        "커밋 메시지가 너무 깁니다.",
        "메시지를 줄인 뒤 다시 시도하세요.",
    ),
    message_character: lines(
        "커밋 메시지에 Git이 쓸 수 없는 문자가 있습니다.",
        "그 문자를 지운 뒤 다시 시도하세요.",
    ),
    message_shape: lines(
        "Git이 이 커밋 메시지를 쓸 수 없습니다.",
        "다른 메시지를 쓰고 다시 시도하세요.",
    ),
    name_empty: lines(
        "브랜치 이름이 비어 있습니다.",
        "계속하려면 브랜치 이름을 입력하세요.",
    ),
    name_long: lines(
        "브랜치 이름이 너무 깁니다.",
        "더 짧은 브랜치 이름을 고르세요.",
    ),
    name_character: lines(
        "브랜치 이름에 Git이 쓸 수 없는 문자가 있습니다.",
        "공백이나 예약된 문장 부호를 지운 뒤 다시 시도하세요.",
    ),
    name_shape: lines(
        "Git이 이 브랜치 이름을 쓸 수 없습니다.",
        "다른 브랜치 이름을 고르세요.",
    ),
    remote_mismatch: lines(
        "목록의 브랜치가 원격 저장소와 맞지 않습니다.",
        "브랜치 목록을 새로 고친 뒤 브랜치를 다시 고르세요.",
    ),
    keys_exhausted: lines(
        "앱의 한도 때문에 Git 동작을 더 실행할 수 없습니다.",
        "Vessel을 완전히 종료하고 다시 실행한 뒤 시도하세요.",
    ),
};

impl Lang {
    const fn git_failures(self) -> &'static Words {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }

    /// A commit message rejected by the wire's constructor. The limit,
    /// character or precise parser rule is retained as detail.
    #[must_use]
    pub fn git_message_failure(self, error: &InvalidText) -> GitFailureText {
        let words = self.git_failures();
        let lines = match error.problem {
            TextProblem::Empty => words.message_empty,
            TextProblem::TooLong { .. } => words.message_long,
            TextProblem::Character { .. } => words.message_character,
            TextProblem::Shape { .. } => words.message_shape,
        };
        self.git_validation(lines, &error.problem)
    }

    /// A branch-name form rejected by the same constructor as the node.
    #[must_use]
    pub fn git_name_failure(self, error: &InvalidText) -> GitFailureText {
        let words = self.git_failures();
        let lines = match error.problem {
            TextProblem::Empty => words.name_empty,
            TextProblem::TooLong { .. } => words.name_long,
            TextProblem::Character { .. } => words.name_character,
            TextProblem::Shape { .. } => words.name_shape,
        };
        self.git_validation(lines, &error.problem)
    }

    /// A listed remote ref that cannot safely become a local branch.
    #[must_use]
    pub fn git_switch_failure(self, error: &SwitchProblem) -> GitFailureText {
        match error {
            SwitchProblem::InvalidName(error) => self.git_name_failure(error),
            SwitchProblem::RemoteMismatch { reference, remote } => {
                let mut text = failure(self.git_failures().remote_mismatch);
                text.detail = Some(match self {
                    Self::En => format!("Listed branch: {reference}. Remote: {remote}."),
                    Self::Ko => format!("목록의 브랜치: {reference}. 원격 저장소: {remote}."),
                });
                text
            }
        }
    }

    /// A local counter exhausted, with a useful action rather than its
    /// implementation-level `Display` text.
    #[must_use]
    pub fn git_keys_exhausted(self) -> GitFailureText {
        failure(self.git_failures().keys_exhausted)
    }

    fn git_validation(self, lines: Lines, problem: &TextProblem) -> GitFailureText {
        let mut text = failure(lines);
        text.detail = match problem {
            TextProblem::Empty => None,
            TextProblem::TooLong { max, .. } => Some(match self {
                Self::En => format!("Maximum length: {}.", self.git_limit(*max)),
                Self::Ko => format!("최대 길이: {}.", self.git_limit(*max)),
            }),
            TextProblem::Character { character, .. } => Some(match self {
                Self::En => format!("Unsupported character: {}.", self.git_character(*character)),
                Self::Ko => format!("쓸 수 없는 문자: {}.", self.git_character(*character)),
            }),
            // The parser's exact rule is diagnostic data, like a hook's
            // output. It is never used as the localized explanation.
            TextProblem::Shape { rule, .. } => Some((*rule).to_owned()),
        };
        text
    }

    fn git_character(self, character: char) -> String {
        match (self, character) {
            (Self::En | Self::Ko, '\0') => "NUL".to_owned(),
            (Self::En, ' ') => "space".to_owned(),
            (Self::Ko, ' ') => "공백".to_owned(),
            (Self::En, '\n') => "line break".to_owned(),
            (Self::Ko, '\n') => "줄바꿈".to_owned(),
            (Self::En, '\t') => "tab".to_owned(),
            (Self::Ko, '\t') => "탭".to_owned(),
            (Self::En | Self::Ko, value) if value.is_control() => {
                format!("U+{:04X}", u32::from(value))
            }
            (Self::En | Self::Ko, value) => format!("‘{value}’"),
        }
    }

    fn git_limit(self, bytes: usize) -> String {
        if bytes >= 1024
            && let Ok(bytes) = u64::try_from(bytes)
        {
            return crate::workspace::size::size_word(bytes);
        }
        match self {
            Self::En => format!("{bytes} bytes"),
            Self::Ko => format!("{bytes}바이트"),
        }
    }
}

fn failure(lines: Lines) -> GitFailureText {
    GitFailureText {
        title: lines.title.to_owned(),
        next: lines.next.to_owned(),
        detail: None,
    }
}
