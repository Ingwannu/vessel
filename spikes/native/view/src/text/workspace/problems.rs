//! Closed-code fallbacks keep redacted C5 failures visible and localized.
//! Existing nonempty node diagnostics retain their legacy rendering contract;
//! the file boundary strips raw paths/credentials before reaching these words.
use crate::Lang;
use crate::workspace::problem::Problem;
use vessel_proto::ErrorCode;

pub(super) fn problem(lang: Lang, problem: &Problem) -> String {
    match (lang, problem) {
        (Lang::En | Lang::Ko, Problem::Node { message, .. }) if !message.is_empty() => {
            message.clone()
        }
        (lang, Problem::Node { code, .. }) => refusal(lang, *code).to_owned(),
        (Lang::En, Problem::Older) => {
            "This machine's Vessel is too old for this; update it to use it here.".to_owned()
        }
        (Lang::Ko, Problem::Older) => {
            "이 머신의 Vessel이 너무 오래돼 쓸 수 없습니다. 업데이트하세요.".to_owned()
        }
        (Lang::En, Problem::Gone(_)) => {
            "The machine could not be reached. Check that it is on and its node is running."
                .to_owned()
        }
        (Lang::Ko, Problem::Gone(_)) => {
            "머신에 연결할 수 없습니다. 머신이 켜져 있고 노드가 실행 중인지 확인하세요.".to_owned()
        }
        (Lang::En, Problem::Unsent) => "Could not send. Try again.".to_owned(),
        (Lang::Ko, Problem::Unsent) => "보내지 못했습니다. 다시 시도하세요.".to_owned(),
    }
}
const fn refusal(lang: Lang, code: ErrorCode) -> &'static str {
    match (lang, code) {
        (Lang::En, ErrorCode::Unauthorized | ErrorCode::Forbidden) => {
            "This app does not have permission for this operation."
        }
        (Lang::Ko, ErrorCode::Unauthorized | ErrorCode::Forbidden) => {
            "이 앱에는 이 동작을 수행할 권한이 없습니다."
        }
        (Lang::En, ErrorCode::ProtocolMismatch | ErrorCode::Unsupported) => {
            "This machine does not support this operation."
        }
        (Lang::Ko, ErrorCode::ProtocolMismatch | ErrorCode::Unsupported) => {
            "이 머신은 이 동작을 지원하지 않습니다."
        }
        (Lang::En, ErrorCode::NotFound) => "The requested item could not be found.",
        (Lang::Ko, ErrorCode::NotFound) => "요청한 항목을 찾을 수 없습니다.",
        (Lang::En, ErrorCode::Conflict) => {
            "The saved item changed. Check its current state before making another change."
        }
        (Lang::Ko, ErrorCode::Conflict) => {
            "저장된 항목이 변경되었습니다. 다시 변경하기 전에 현재 상태를 확인하세요."
        }
        (Lang::En, ErrorCode::InvalidRequest) => "The machine refused this request.",
        (Lang::Ko, ErrorCode::InvalidRequest) => "머신이 이 요청을 거부했습니다.",
        (
            Lang::En,
            ErrorCode::HarnessNotInstalled
            | ErrorCode::SpawnFailed
            | ErrorCode::Unavailable
            | ErrorCode::Internal,
        ) => "The machine did not confirm completion of this operation.",
        (
            Lang::Ko,
            ErrorCode::HarnessNotInstalled
            | ErrorCode::SpawnFailed
            | ErrorCode::Unavailable
            | ErrorCode::Internal,
        ) => "머신에서 이 동작의 완료를 확인하지 못했습니다.",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_closed_code_has_nonempty_localized_copy_without_an_automatic_retry_claim() {
        for code in [
            ErrorCode::InvalidRequest,
            ErrorCode::Unauthorized,
            ErrorCode::Forbidden,
            ErrorCode::ProtocolMismatch,
            ErrorCode::NotFound,
            ErrorCode::Conflict,
            ErrorCode::Unsupported,
            ErrorCode::HarnessNotInstalled,
            ErrorCode::SpawnFailed,
            ErrorCode::Unavailable,
            ErrorCode::Internal,
        ] {
            let closed = Problem::Node {
                code,
                message: String::new(),
            };
            let en = problem(Lang::En, &closed);
            let ko = problem(Lang::Ko, &closed);
            assert!(!en.is_empty());
            assert!(!ko.is_empty());
            assert_ne!(en, ko);
            assert!(!en.contains("Try again"));
        }
    }
    #[test]
    fn transport_details_remain_hidden_and_nonempty_legacy_node_copy_stays_unchanged() {
        let gone = Problem::Gone("SENSITIVE_FIXTURE_ENDPOINT".into());
        for lang in [Lang::En, Lang::Ko] {
            assert!(!problem(lang, &gone).contains("SENSITIVE_FIXTURE_ENDPOINT"));
            assert_eq!(
                problem(
                    lang,
                    &Problem::Node {
                        code: ErrorCode::Conflict,
                        message: "legacy diagnostic".into()
                    }
                ),
                "legacy diagnostic"
            );
        }
    }
}
