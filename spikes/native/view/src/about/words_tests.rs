//! Whole-sentence parity with the retired English/Korean table.

use crate::about::Role;
use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};

const ROLE_SENTENCES: &[(Role, &str, &str)] = &[
    (Role::Drawing, "Drawing the windows", "창 그리기"),
    (Role::Windows, "The platform's windows", "플랫폼의 창"),
    (Role::Editing, "Text editing", "글 편집"),
    (Role::Controls, "Controls", "컨트롤"),
    (Role::Bytes, "Bytes on the wire", "주고받는 바이트"),
    (Role::Dates, "Dates and times", "날짜와 시간"),
    (Role::Folders, "The platform's folders", "플랫폼의 폴더"),
    (
        Role::ComputerName,
        "This computer's name",
        "이 컴퓨터의 이름",
    ),
    (Role::Data, "Reading and writing data", "데이터 읽고 쓰기"),
    (Role::Translations, "Translations", "번역"),
    (Role::Identifiers, "Unique identifiers", "고유 식별자"),
    (Role::Language, "The system's language", "시스템 언어"),
    (Role::Desktop, "Desktop settings", "데스크톱 설정"),
    (Role::Tasks, "Asynchronous tasks", "비동기 작업"),
    (Role::Images, "Drawing images", "이미지 그리기"),
    (Role::Errors, "Errors", "오류"),
    (Role::AudioDevices, "Audio devices", "오디오 장치"),
    (Role::AudioProcessing, "Audio processing", "오디오 처리"),
    (
        Role::Network,
        "The network and its tasks",
        "네트워크와 그 작업",
    ),
    (Role::Logs, "Logs", "로그"),
    (Role::QrCodes, "Pairing QR codes", "페어링 QR 코드"),
    (
        Role::Terminal,
        "Reading a terminal program's output",
        "터미널 프로그램의 출력 읽기",
    ),
    (
        Role::Accents,
        "Find's folding of accents",
        "찾기의 악센트 접기",
    ),
    (Role::Face, "Geist, the app's face", "Geist, 앱의 글꼴"),
    (
        Role::CodeFace,
        "Geist Mono, for code and ids",
        "Geist Mono, 코드와 ID용",
    ),
    (Role::Hangul, "Pretendard, for Hangul", "Pretendard, 한글용"),
];

#[test]
fn english_and_korean_labels_keep_the_table_sentences() {
    let en = Translator::get(ENGLISH).unwrap().about();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .about();
    assert_eq!(en.licence(), "Licence");
    assert_eq!(ko.licence(), "라이선스");
    assert_eq!(en.licence_note(), "Source-available; see LICENSE.md.");
    assert_eq!(ko.licence_note(), "소스 공개. LICENSE.md를 보세요.");
    assert_eq!(en.software(), "Software in this app");
    assert_eq!(ko.software(), "이 앱에 든 소프트웨어");
    for &(role, english, korean) in ROLE_SENTENCES {
        assert_eq!(en.role(role), english);
        assert_eq!(ko.role(role), korean);
    }
}
