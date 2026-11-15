//! Fixed interface sentences in both languages; report data is unaffected.
use crate::localization::{Locale, Translator};

#[test]
fn controls_keep_the_existing_english_and_korean_sentences() {
    let en = Translator::get(Locale::from_tag("en").unwrap())
        .unwrap()
        .diagnostics();
    let ko = Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .diagnostics();
    assert_eq!(en.title(), "Diagnostics");
    assert_eq!(ko.title(), "진단");
    assert_eq!(en.report_title(), "App diagnostic");
    assert_eq!(ko.report_title(), "앱 진단");
    assert_eq!(
        en.description(),
        "Collect connection, machine, agent, and log details"
    );
    assert_eq!(
        ko.description(),
        "연결, 머신, 에이전트 및 로그 세부 정보를 수집합니다."
    );
    assert_eq!(en.run(), "Run");
    assert_eq!(ko.run(), "실행");
    assert_eq!(en.copy(), "Copy");
    assert_eq!(ko.copy(), "복사");
    assert_eq!(en.refresh(), "Refresh");
    assert_eq!(ko.refresh(), "새로 고침");
    assert_eq!(en.copied(), "Diagnostic copied.");
    assert_eq!(ko.copied(), "진단을 복사했습니다.");
}

#[test]
fn supported_collection_framing_matches_its_data_scope_in_both_languages() {
    let en = Translator::get(Locale::from_tag("en").unwrap())
        .unwrap()
        .diagnostic_flow();
    let ko = Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .diagnostic_flow();
    assert_eq!(
        en.daemon_note(),
        "Basic host information and the daemon report are collected independently."
    );
    assert_eq!(
        ko.daemon_note(),
        "기본 머신 정보와 데몬 보고서는 각각 수집합니다."
    );
    assert_eq!(
        en.process_note(),
        "Only the daemon’s own numeric process metadata is collected. Unsupported sources stay explicit."
    );
    assert_eq!(
        ko.process_note(),
        "데몬 자체의 숫자형 프로세스 메타데이터만 수집합니다. 지원하지 않는 항목은 별도로 표시합니다."
    );
    assert_eq!(
        en.logs_note(),
        "Raw daemon logs are not collected. The report includes only a bounded summary of failed node calls."
    );
    assert_eq!(
        ko.logs_note(),
        "원본 데몬 로그는 수집하지 않습니다. 보고서에는 실패한 노드 호출의 제한된 요약만 포함합니다."
    );
    assert_eq!(
        en.privacy(),
        "Reports use allowlisted host metadata. Host names, routes, keys, paths and raw errors are excluded."
    );
    assert_eq!(
        ko.privacy(),
        "보고서는 허용된 머신 메타데이터만 사용합니다. 머신 이름, 접속 경로, 키, 파일 경로, 원본 오류는 제외합니다."
    );
    assert_eq!(en.partial(), "Host diagnostics");
    assert_eq!(ko.partial(), "머신 진단");
    assert_eq!(en.host_dependent(), "Depends on host");
    assert_eq!(ko.host_dependent(), "머신에 따라 다름");
}
