//! Error actions and reasons are complete sentences, never peer text.
use super::{Lang, SyncWords};
use crate::sync::{Part, Problem};
use vessel_proto::ErrorCode;

impl SyncWords {
    /// Per-read failure beside that section, with a complete next step.
    /// `part` names an action, `problem` contains codes only; this wraps.
    #[must_use]
    pub fn problem(&self, part: Part, problem: Problem) -> String {
        let why = self.reason(problem);
        match (self.lang, part) {
            (Lang::En, Part::Overview) => format!("Could not read sharing status. {why}"),
            (Lang::Ko, Part::Overview) => format!("공유 상태를 읽지 못했습니다. {why}"),
            (Lang::En, Part::Publication) => format!("Could not read shared configuration. {why}"),
            (Lang::Ko, Part::Publication) => format!("공유 설정을 읽지 못했습니다. {why}"),
            (Lang::En, Part::Members) => format!("Could not read member reports. {why}"),
            (Lang::Ko, Part::Members) => format!("구성원의 보고를 읽지 못했습니다. {why}"),
            (Lang::En, Part::Connections) => format!("Could not read machine connections. {why}"),
            (Lang::Ko, Part::Connections) => format!("머신 연결을 읽지 못했습니다. {why}"),
            (Lang::En, Part::Secrets) => format!("Could not read stored secret names. {why}"),
            (Lang::Ko, Part::Secrets) => format!("저장된 비밀값 이름을 읽지 못했습니다. {why}"),
        }
    }
    /// A safe complete reason and next step; no wire message is accepted.
    #[must_use]
    pub const fn reason(&self, problem: Problem) -> &'static str {
        match (self.lang, problem) {
            (Lang::En, Problem::Offline) => "Reconnect the machine and try again.",
            (Lang::Ko, Problem::Offline) => "머신을 다시 연결한 뒤 시도하세요.",
            (Lang::En, Problem::InvalidReply) => {
                "The machine returned an invalid response. Try again."
            }
            (Lang::Ko, Problem::InvalidReply) => {
                "머신의 응답이 올바르지 않습니다. 다시 시도하세요."
            }
            (Lang::En, Problem::Exhausted) => "Reopen the app to continue safely.",
            (Lang::Ko, Problem::Exhausted) => "계속하려면 앱을 다시 여세요.",
            (Lang::En, Problem::Refused(ErrorCode::Unauthorized | ErrorCode::Forbidden)) => {
                "This device cannot read this shared machine information. Use an administrator device."
            }
            (Lang::Ko, Problem::Refused(ErrorCode::Unauthorized | ErrorCode::Forbidden)) => {
                "이 기기에는 이 머신 공유 정보를 읽을 권한이 없습니다. 관리자 기기를 사용하세요."
            }
            (Lang::En, Problem::Refused(ErrorCode::ProtocolMismatch | ErrorCode::Unsupported)) => {
                "This connection cannot provide this information. Try a direct connection to the machine. If it is already direct, update the app and the machine."
            }
            (Lang::Ko, Problem::Refused(ErrorCode::ProtocolMismatch | ErrorCode::Unsupported)) => {
                "이 연결에서는 정보를 제공할 수 없습니다. 머신에 직접 연결해 보세요. 이미 직접 연결했다면 앱과 머신을 업데이트하세요."
            }
            (Lang::En, Problem::Refused(ErrorCode::NotFound)) => {
                "That shared machine information is no longer available. Refresh this page."
            }
            (Lang::Ko, Problem::Refused(ErrorCode::NotFound)) => {
                "머신 공유 정보가 더 이상 존재하지 않습니다. 이 페이지를 새로고침하세요."
            }
            (Lang::En, Problem::Refused(ErrorCode::InvalidRequest | ErrorCode::Unavailable)) => {
                "Check the machine's sharing setup and try again."
            }
            (Lang::Ko, Problem::Refused(ErrorCode::InvalidRequest | ErrorCode::Unavailable)) => {
                "공유 설정을 확인한 뒤 다시 시도하세요."
            }
            (
                Lang::En,
                Problem::Refused(
                    ErrorCode::Conflict
                    | ErrorCode::HarnessNotInstalled
                    | ErrorCode::SpawnFailed
                    | ErrorCode::Internal,
                ),
            ) => "The read did not complete. Try again.",
            (
                Lang::Ko,
                Problem::Refused(
                    ErrorCode::Conflict
                    | ErrorCode::HarnessNotInstalled
                    | ErrorCode::SpawnFailed
                    | ErrorCode::Internal,
                ),
            ) => "읽기를 완료하지 못했습니다. 다시 시도하세요.",
        }
    }
}
