//! The words for one of the app's files that does not read (the machine
//! list, the preferences): where and what, in the app's words, never the
//! file's text (`crate::files`). See `text/mod.rs` for the convention.

use super::Lang;
use crate::files::{FileFault, FileProblem};

impl Lang {
    /// Where in the file and what is wrong, as one clause (used inside a
    /// sentence that names the file).
    #[must_use]
    pub fn file_problem(self, fault: &FileFault) -> String {
        let what = self.problem(fault);
        match (self, fault.at) {
            (Self::En, Some((line, column))) => format!("line {line}, column {column}: {what}"),
            (Self::Ko, Some((line, column))) => format!("{line}번째 줄 {column}번째 칸: {what}"),
            (_, None) => what,
        }
    }

    /// The frame's foot when the machine list does not read.
    #[must_use]
    pub fn machines_unreadable(self, fault: &FileFault) -> String {
        let path = &fault.path;
        let problem = self.file_problem(fault);
        match self {
            Self::En => {
                format!("The machine list {path} does not read ({problem}). Fix it, then reload.")
            }
            Self::Ko => format!(
                "머신 목록 파일을 읽지 못했습니다: {path} ({problem}). 고친 뒤 새로고침하세요."
            ),
        }
    }

    /// What is wrong, in the app's words: a key is named only when the
    /// reader found one of the file's own keys there.
    fn problem(self, fault: &FileFault) -> String {
        let field = fault.field.as_deref();
        match (self, fault.problem, field) {
            (Self::En, FileProblem::NotToml, _) => {
                "this is not TOML (look for an unclosed quote or bracket, or a line without `=`)"
                    .to_owned()
            }
            (Self::Ko, FileProblem::NotToml, _) => {
                "TOML 형식이 아닙니다(닫히지 않은 따옴표나 괄호, `=`가 없는 줄을 확인하세요)"
                    .to_owned()
            }
            (Self::En, FileProblem::NotJson, _) => "this is not JSON".to_owned(),
            (Self::Ko, FileProblem::NotJson, _) => "JSON 형식이 아닙니다".to_owned(),
            (Self::En, FileProblem::UnknownKey, _) => "a key this file does not take".to_owned(),
            (Self::Ko, FileProblem::UnknownKey, _) => {
                "이 파일에서 쓰지 않는 키가 있습니다".to_owned()
            }
            (Self::En, FileProblem::DuplicateKey, _) => "a key given twice".to_owned(),
            (Self::Ko, FileProblem::DuplicateKey, _) => "같은 키가 두 번 있습니다".to_owned(),
            (Self::En, FileProblem::MissingKey, _) => "a key this file needs is missing".to_owned(),
            (Self::Ko, FileProblem::MissingKey, _) => "꼭 있어야 하는 키가 없습니다".to_owned(),
            (Self::En, FileProblem::WrongKind, Some("token")) => {
                "`token` names the file that holds the token, token = { file = \"…\" }, \
                 never the token itself"
                    .to_owned()
            }
            (Self::Ko, FileProblem::WrongKind, Some("token")) => {
                "`token`에는 토큰 자체가 아니라 토큰이 든 파일을 적습니다: token = { file = \"…\" }"
                    .to_owned()
            }
            (Self::En, FileProblem::WrongKind, Some(field)) => {
                format!("`{field}` has a value of the wrong kind")
            }
            (Self::Ko, FileProblem::WrongKind, Some(field)) => {
                format!("`{field}` 값의 종류가 맞지 않습니다")
            }
            (Self::En, FileProblem::WrongKind, None) => "a value of the wrong kind".to_owned(),
            (Self::Ko, FileProblem::WrongKind, None) => "값의 종류가 맞지 않습니다".to_owned(),
            (Self::En, FileProblem::CannotOpen, _) => {
                "it cannot be opened (check who owns it and its mode)".to_owned()
            }
            (Self::Ko, FileProblem::CannotOpen, _) => {
                "파일을 열 수 없습니다(소유자와 권한을 확인하세요)".to_owned()
            }
        }
    }
}
