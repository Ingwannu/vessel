//! Whole error messages for every action and safe code in both languages.
use crate::{
    Lang,
    sync::{Part, Problem},
};
use vessel_proto::ErrorCode;

const CASES: [(Problem, &str, &str); 14] = [
    (
        Problem::Offline,
        "Reconnect the machine and try again.",
        "머신을 다시 연결한 뒤 시도하세요.",
    ),
    (
        Problem::InvalidReply,
        "The machine returned an invalid response. Try again.",
        "머신의 응답이 올바르지 않습니다. 다시 시도하세요.",
    ),
    (
        Problem::Exhausted,
        "Reopen the app to continue safely.",
        "계속하려면 앱을 다시 여세요.",
    ),
    (
        Problem::Refused(ErrorCode::Unauthorized),
        "This device cannot read this shared machine information. Use an administrator device.",
        "이 기기에는 이 머신 공유 정보를 읽을 권한이 없습니다. 관리자 기기를 사용하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Forbidden),
        "This device cannot read this shared machine information. Use an administrator device.",
        "이 기기에는 이 머신 공유 정보를 읽을 권한이 없습니다. 관리자 기기를 사용하세요.",
    ),
    (
        Problem::Refused(ErrorCode::ProtocolMismatch),
        "This connection cannot provide this information. Try a direct connection to the machine. If it is already direct, update the app and the machine.",
        "이 연결에서는 정보를 제공할 수 없습니다. 머신에 직접 연결해 보세요. 이미 직접 연결했다면 앱과 머신을 업데이트하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Unsupported),
        "This connection cannot provide this information. Try a direct connection to the machine. If it is already direct, update the app and the machine.",
        "이 연결에서는 정보를 제공할 수 없습니다. 머신에 직접 연결해 보세요. 이미 직접 연결했다면 앱과 머신을 업데이트하세요.",
    ),
    (
        Problem::Refused(ErrorCode::NotFound),
        "That shared machine information is no longer available. Refresh this page.",
        "머신 공유 정보가 더 이상 존재하지 않습니다. 이 페이지를 새로고침하세요.",
    ),
    (
        Problem::Refused(ErrorCode::InvalidRequest),
        "Check the machine's sharing setup and try again.",
        "공유 설정을 확인한 뒤 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Unavailable),
        "Check the machine's sharing setup and try again.",
        "공유 설정을 확인한 뒤 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Conflict),
        "The read did not complete. Try again.",
        "읽기를 완료하지 못했습니다. 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::HarnessNotInstalled),
        "The read did not complete. Try again.",
        "읽기를 완료하지 못했습니다. 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::SpawnFailed),
        "The read did not complete. Try again.",
        "읽기를 완료하지 못했습니다. 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Internal),
        "The read did not complete. Try again.",
        "읽기를 완료하지 못했습니다. 다시 시도하세요.",
    ),
];

#[test]
fn every_action_and_reason_is_compared_as_a_whole_message_not_a_prefix() {
    for (problem, en, ko) in CASES {
        assert_eq!(Lang::En.sync().reason(problem), en);
        assert_eq!(Lang::Ko.sync().reason(problem), ko);
        for (part, en_full, ko_full) in [
            (
                Part::Overview,
                format!("Could not read sharing status. {en}"),
                format!("공유 상태를 읽지 못했습니다. {ko}"),
            ),
            (
                Part::Publication,
                format!("Could not read shared configuration. {en}"),
                format!("공유 설정을 읽지 못했습니다. {ko}"),
            ),
            (
                Part::Members,
                format!("Could not read member reports. {en}"),
                format!("구성원의 보고를 읽지 못했습니다. {ko}"),
            ),
            (
                Part::Connections,
                format!("Could not read machine connections. {en}"),
                format!("머신 연결을 읽지 못했습니다. {ko}"),
            ),
            (
                Part::Secrets,
                format!("Could not read stored secret names. {en}"),
                format!("저장된 비밀값 이름을 읽지 못했습니다. {ko}"),
            ),
        ] {
            assert_eq!(Lang::En.sync().problem(part, problem), en_full);
            assert_eq!(Lang::Ko.sync().problem(part, problem), ko_full);
        }
    }
}
