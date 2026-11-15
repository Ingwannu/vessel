use super::*;
use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};

#[test]
fn every_offer_has_whole_english_and_korean_help() {
    let en = Translator::get(ENGLISH).unwrap().agent_catalog();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_catalog();
    let agent = HarnessRef::Added("example".parse().unwrap());
    let archive: HttpUrl = "https://example.invalid/archive.zip".parse().unwrap();
    for (offer, english, korean) in [
        (
            Offer::Recorded(&agent),
            "This provider is already added on this machine.",
            "이 머신에 이미 추가된 프로바이더입니다.",
        ),
        (
            Offer::Add(Runner::Found),
            "Select “Add” to record this provider. It does not install or start the provider.",
            "프로바이더를 등록하려면 “추가” 버튼을 누르세요. 프로바이더를 설치하거나 실행하지는 않습니다.",
        ),
        (
            Offer::Add(Runner::Missing),
            "You can add this provider, but its package runner must be installed on the machine before you can chat.",
            "프로바이더를 추가할 수 있지만, 대화하려면 먼저 머신에 패키지 실행기를 설치해야 합니다.",
        ),
        (
            Offer::BuiltIn(HarnessKind::Codex),
            "This built-in provider is managed separately.",
            "기본 제공 프로바이더로 별도 관리되는 항목입니다.",
        ),
        (
            Offer::Download(&archive),
            "Install this provider on the machine, then add it by its command.",
            "머신에 프로바이더를 설치한 다음 명령으로 추가하세요.",
        ),
        (
            Offer::Unavailable,
            "This provider is not available for this machine.",
            "이 머신에서는 사용할 수 없는 프로바이더입니다.",
        ),
    ] {
        assert_eq!(en.offer(&offer), english);
        assert_eq!(ko.offer(&offer), korean);
    }
}

#[test]
fn control_and_empty_state_words_are_complete_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap().agent_catalog();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_catalog();
    for (actual, expected) in [
        (en.title(), "Add provider"),
        (ko.title(), "프로바이더 추가"),
        (en.search(), "Search providers"),
        (ko.search(), "프로바이더 검색"),
        (en.add(), "Add"),
        (ko.add(), "추가"),
        (en.adding(), "Adding…"),
        (ko.adding(), "추가 중…"),
        (
            en.empty(),
            "No registry providers were listed by this machine.",
        ),
        (
            ko.empty(),
            "이 머신에서 표시할 레지스트리 프로바이더가 없습니다.",
        ),
        (en.no_matches(), "No providers match your search."),
        (ko.no_matches(), "검색과 일치하는 프로바이더가 없습니다."),
        (
            en.not_read(),
            "The provider registry has not been read yet.",
        ),
        (
            ko.not_read(),
            "아직 프로바이더 레지스트리를 읽지 않았습니다.",
        ),
        (en.reading(), "Reading the provider registry…"),
        (ko.reading(), "프로바이더 레지스트리를 읽는 중…"),
    ] {
        assert_eq!(actual, expected);
    }
}

#[test]
fn registry_failures_have_whole_instructions_and_reference_tones() {
    use crate::agent_status::catalogue::Problem;
    use crate::alert::Variant;
    let en = Translator::get(ENGLISH).unwrap().agent_catalog();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_catalog();
    assert_eq!(en.refresh(), "Refresh");
    assert_eq!(ko.refresh(), "새로 고침");
    for (problem, english, korean, tone) in [
        (
            Problem::NodeChanged,
            "The machine behind this name changed. Select “Refresh” to read its provider list before trying again.",
            "이 이름이 가리키는 머신이 변경되었습니다. 다시 시도하기 전에 “새로 고침” 버튼을 선택해 프로바이더 목록을 읽으세요.",
            Variant::Warning,
        ),
        (
            Problem::Busy,
            "Settings is busy. Try again when the current operation finishes.",
            "설정 작업이 진행 중입니다. 현재 작업이 끝난 뒤 다시 시도하세요.",
            Variant::Warning,
        ),
        (
            Problem::Offline,
            "The machine is not connected. Connect it, then select “Refresh”.",
            "머신에 연결되지 않았습니다. 연결한 뒤 “새로 고침” 버튼을 선택하세요.",
            Variant::Error,
        ),
        (
            Problem::Unsupported,
            "Update Vessel on this machine to read the provider registry.",
            "프로바이더 레지스트리를 읽으려면 이 머신의 Vessel을 업데이트하세요.",
            Variant::Warning,
        ),
        (
            Problem::Permission,
            "This device does not have permission for this registry action. Ask the machine’s administrator to grant access.",
            "이 기기에는 이 레지스트리 작업을 수행할 권한이 없습니다. 머신 관리자에게 접근 권한을 요청하세요.",
            Variant::Warning,
        ),
        (
            Problem::Timeout,
            "The machine did not return the provider registry in time. Select “Refresh” to try again.",
            "머신이 제한 시간 안에 프로바이더 레지스트리를 보내지 않았습니다. “새로 고침” 버튼을 선택해 다시 시도하세요.",
            Variant::Error,
        ),
        (
            Problem::Read,
            "Could not read the provider registry. Select “Refresh” to try again.",
            "프로바이더 레지스트리를 읽지 못했습니다. “새로 고침” 버튼을 선택해 다시 시도하세요.",
            Variant::Error,
        ),
        (
            Problem::NoTransport,
            "This browser cannot connect to a machine yet.",
            "이 브라우저에서는 아직 머신에 연결할 수 없습니다.",
            Variant::Warning,
        ),
    ] {
        assert_eq!(en.problem(problem), english);
        assert_eq!(ko.problem(problem), korean);
        assert_eq!(problem.variant(), tone);
    }
}
