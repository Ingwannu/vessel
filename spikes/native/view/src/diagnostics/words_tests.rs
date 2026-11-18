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
    // The label in Paseo's "Copied {label}" toast (settings.diagnostics.app.copyLabel).
    assert_eq!(en.copied_label(), "diagnostic");
    assert_eq!(ko.copied_label(), "진단");
}
