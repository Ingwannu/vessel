//! Machine book's committed-success boundary, following the page's book edits.
use crate::Lang;
use crate::machine_book::{Edit, Failure, Name};
fn name(text: &str) -> Name {
    Name::parse(text).unwrap()
}
#[test]
fn names_are_checked_before_a_command_can_be_built() {
    for text in [
        "",
        " leading",
        "a b",
        ".hidden",
        "한글",
        "a/b",
        &"a".repeat(65),
    ] {
        assert!(Name::parse(text).is_err());
    }
    for text in ["a", "0", "a.b_c-d", &"a".repeat(64)] {
        assert_eq!(Name::parse(text).unwrap().as_str(), text);
    }
}
#[test]
fn selection_follows_only_the_edited_machine() {
    let rename = Edit::Rename {
        from: name("old"),
        to: name("new"),
    };
    assert_eq!(rename.selected_after(Some("old")), Some("new".to_owned()));
    assert_eq!(
        rename.selected_after(Some("other")),
        Some("other".to_owned())
    );
    assert_eq!(rename.selected_after(None), None);
    let forget = Edit::Forget(name("old"));
    assert_eq!(forget.selected_after(Some("old")), None);
    assert_eq!(
        forget.selected_after(Some("other")),
        Some("other".to_owned())
    );
    assert_eq!(
        Edit::MakeDefault(name("new")).selected_after(Some("old")),
        Some("old".to_owned())
    );
}
#[test]
fn errors_are_whole_localized_sentences_without_raw_values() {
    let cases = [
        (
            Failure::NoConfiguration,
            "The configuration folder could not be found. Choose a configuration folder and restart Vessel.",
            "설정 폴더를 찾을 수 없습니다. 설정 폴더를 지정하고 Vessel을 다시 시작하세요.",
        ),
        (
            Failure::Read,
            "The machine list could not be read. Check that your account can read the configuration folder, then try again.",
            "머신 목록을 읽을 수 없습니다. 현재 계정의 설정 폴더 읽기 권한을 확인하고 다시 시도하세요.",
        ),
        (
            Failure::InvalidBook,
            "The machine list has an invalid entry. Correct the node book, then try again.",
            "머신 목록에 잘못된 항목이 있습니다. 노드 목록 파일을 고치고 다시 시도하세요.",
        ),
        (
            Failure::Missing,
            "This machine is no longer in the list. Refresh the list and choose another machine.",
            "이 머신은 더 이상 목록에 없습니다. 목록을 새로 고치고 다른 머신을 선택하세요.",
        ),
        (
            Failure::Taken,
            "Another machine already has that name. Choose a different name.",
            "같은 이름의 머신이 이미 있습니다. 다른 이름을 입력하세요.",
        ),
        (
            Failure::Write,
            "The change could not be saved. Check that your account can write to the configuration folder, then try again.",
            "변경 사항을 저장할 수 없습니다. 현재 계정의 설정 폴더 쓰기 권한을 확인하고 다시 시도하세요.",
        ),
        (
            Failure::Worker,
            "The file operation stopped before it finished. Try again.",
            "파일 작업이 완료되기 전에 중단되었습니다. 다시 시도하세요.",
        ),
        (
            Failure::Unavailable,
            "Machine management is available in the desktop app.",
            "머신 관리는 데스크톱 앱에서 사용할 수 있습니다.",
        ),
    ];
    for (failure, en, ko) in cases {
        assert_eq!(Lang::En.machine_book_failure(failure), en);
        assert_eq!(Lang::Ko.machine_book_failure(failure), ko);
    }
}
