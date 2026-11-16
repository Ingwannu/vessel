//! Whole-sentence goldens keep disclosure scope and Fluent isolation together.
use super::*;
use crate::localization::{ENGLISH, Locale, Translator};
use vessel_proto::usage_reports::{
    UsageCredentialStore, UsageDisclosureVersion, UsageRecipient, UsageSourceDisclosure,
};

#[test]
fn source_disclosure_names_translated_stores_and_recipients_without_credentials() {
    let disclosure = UsageSourceDisclosure {
        version: UsageDisclosureVersion::INITIAL,
        store: UsageCredentialStore::Cursor,
        recipient: UsageRecipient::Cursor,
    };
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    assert_eq!(
        source_disclosure(&disclosure, en),
        "An explicit refresh may read \u{2068}Cursor environment credentials, its read-only desktop sign-in database, or its headless sign-in\u{2069} on this machine and send that sign-in to \u{2068}Cursor\u{2069}. Enabling access does not fetch usage."
    );
    assert_eq!(
        source_disclosure(&disclosure, ko),
        "명시적으로 새로 고침하면 이 머신의 \u{2068}Cursor 환경 자격 증명, 읽기 전용 데스크톱 로그인 데이터베이스 또는 헤드리스 로그인\u{2069}\u{2068}을\u{2069} 읽어 \u{2068}Cursor\u{2069}에 전달할 수 있습니다. 접근을 켜는 것만으로 조회하지 않습니다."
    );
}

#[test]
fn turn_coverage_is_one_whole_label_without_a_singular_turns_error() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    assert_eq!(
        en.usage().turn_coverage("1", "1").unwrap(),
        "Turns reported: \u{2068}1\u{2069} / \u{2068}1\u{2069}"
    );
    assert_eq!(
        ko.usage().turn_coverage("1", "2").unwrap(),
        "보고된 턴: \u{2068}1\u{2069} / \u{2068}2\u{2069}"
    );
}

#[test]
fn korean_machine_and_client_device_remain_distinct() {
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let text = ko.usage();
    assert_eq!(
        text.lead(),
        "이 머신의 에이전트가 보고한 토큰과 비용입니다. 누락된 사용량은 추정하지 않습니다."
    );
    assert_eq!(text.host(), "머신");
    assert_eq!(text.no_hosts(), "연결된 머신이 없습니다.");
    assert_eq!(
        text.forbidden(),
        "이 기기에 이 작업을 수행할 권한이 없습니다."
    );
    assert_eq!(text.withheld(), "이 기기에 숨겨짐");
    assert_eq!(text.allowed(), "공급자가 사용을 허용함");
    assert_eq!(text.limit_reached(), "공급자가 한도 도달을 보고함");
    assert_eq!(text.refresh(), "새로 고침");
}
