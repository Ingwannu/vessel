//! The words of Settings › About beyond the app's updates (those are
//! `text/update.rs`): Vessel's licence and the parts the app is built
//! from, in English and Korean. The page's words (`features/settings/
//! about.tsx`), "this page" made "this app".

use super::Lang;
use crate::about::Role;

impl Lang {
    /// The words of About's licences in this language.
    #[must_use]
    pub const fn about(self) -> &'static AboutText {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct AboutText {
    pub lang: Lang,
    pub licence: &'static str,
    pub licence_note: &'static str,
    pub software: &'static str,
}

const EN: AboutText = AboutText {
    lang: Lang::En,
    licence: "Licence",
    licence_note: "Source-available; see LICENSE.md.",
    software: "Software in this app",
};

const KO: AboutText = AboutText {
    lang: Lang::Ko,
    licence: "라이선스",
    licence_note: "소스 공개. LICENSE.md를 보세요.",
    software: "이 앱에 든 소프트웨어",
};

impl AboutText {
    /// What a part does in the app.
    #[must_use]
    pub const fn role(&self, role: Role) -> &'static str {
        match (self.lang, role) {
            (Lang::En, Role::Drawing) => "Drawing the windows",
            (Lang::Ko, Role::Drawing) => "창 그리기",
            (Lang::En, Role::Windows) => "The platform's windows",
            (Lang::Ko, Role::Windows) => "플랫폼의 창",
            (Lang::En, Role::Editing) => "Text editing",
            (Lang::Ko, Role::Editing) => "글 편집",
            (Lang::En, Role::Controls) => "Controls",
            (Lang::Ko, Role::Controls) => "컨트롤",
            (Lang::En, Role::Bytes) => "Bytes on the wire",
            (Lang::Ko, Role::Bytes) => "주고받는 바이트",
            (Lang::En, Role::Dates) => "Dates and times",
            (Lang::Ko, Role::Dates) => "날짜와 시간",
            (Lang::En, Role::Folders) => "The platform's folders",
            (Lang::Ko, Role::Folders) => "플랫폼의 폴더",
            (Lang::En, Role::ComputerName) => "This computer's name",
            (Lang::Ko, Role::ComputerName) => "이 컴퓨터의 이름",
            (Lang::En, Role::Data) => "Reading and writing data",
            (Lang::Ko, Role::Data) => "데이터 읽고 쓰기",
            (Lang::En, Role::Language) => "The system's language",
            (Lang::Ko, Role::Language) => "시스템 언어",
            (Lang::En, Role::Errors) => "Errors",
            (Lang::Ko, Role::Errors) => "오류",
            (Lang::En, Role::Network) => "The network and its tasks",
            (Lang::Ko, Role::Network) => "네트워크와 그 작업",
            (Lang::En, Role::Logs) => "Logs",
            (Lang::Ko, Role::Logs) => "로그",
            (Lang::En, Role::QrCodes) => "Pairing QR codes",
            (Lang::Ko, Role::QrCodes) => "페어링 QR 코드",
            (Lang::En, Role::Accents) => "Find's folding of accents",
            (Lang::Ko, Role::Accents) => "찾기의 악센트 접기",
            (Lang::En, Role::Face) => "Geist, the app's face",
            (Lang::Ko, Role::Face) => "Geist, 앱의 글꼴",
            (Lang::En, Role::CodeFace) => "Geist Mono, for code and ids",
            (Lang::Ko, Role::CodeFace) => "Geist Mono, 코드와 ID용",
            (Lang::En, Role::Hangul) => "Pretendard, for Hangul",
            (Lang::Ko, Role::Hangul) => "Pretendard, 한글용",
        }
    }
}
