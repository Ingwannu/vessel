use super::*;
use crate::Lang;
use crate::workspace::problem::Problem;
use vessel_proto::ErrorCode;

#[test]
fn every_call_code_has_both_guidance_sentences_and_retains_the_node_detail() {
    let cases = [
        (
            ErrorCode::InvalidRequest,
            "The machine could not complete this Git action.",
            "Read the details below, correct the problem, and try again.",
            "머신이 이 Git 동작을 완료하지 못했습니다.",
            "아래 상세 출력을 읽고 문제를 고친 뒤 다시 시도하세요.",
        ),
        (
            ErrorCode::Unauthorized,
            "The machine did not accept this app's sign-in.",
            "Pair this app with the machine again, then try again.",
            "머신이 이 앱의 로그인을 받아들이지 않았습니다.",
            "이 앱과 머신을 다시 페어링한 뒤 시도하세요.",
        ),
        (
            ErrorCode::Forbidden,
            "This app does not have permission for this Git action.",
            "Pair again with permission to change the repository.",
            "이 앱에 이 Git 동작의 권한이 없습니다.",
            "저장소를 변경할 권한으로 다시 페어링하세요.",
        ),
        (
            ErrorCode::ProtocolMismatch,
            "This version of the machine cannot handle this Git action.",
            "Update Vessel on this machine and in the app, then reconnect.",
            "이 머신의 버전은 이 Git 동작을 처리하지 못합니다.",
            "머신과 앱의 Vessel을 업데이트한 뒤 다시 연결하세요.",
        ),
        (
            ErrorCode::NotFound,
            "The repository could not be found on this machine.",
            "Check the project's folder in Settings, then refresh this view.",
            "이 머신에서 저장소를 찾지 못했습니다.",
            "설정에서 프로젝트 폴더를 확인한 뒤 이 화면을 새로 고치세요.",
        ),
        (
            ErrorCode::Conflict,
            "The repository changed before this action could finish.",
            "Refresh its status, check your changes, and try again.",
            "동작이 끝나기 전에 저장소가 바뀌었습니다.",
            "상태를 새로 읽고 변경을 확인한 뒤 다시 시도하세요.",
        ),
        (
            ErrorCode::Unsupported,
            "Git cannot perform this action in this folder.",
            "Check that Git is installed and the folder is a Git repository.",
            "Git이 이 폴더에서 이 동작을 수행할 수 없습니다.",
            "Git이 설치되어 있고 이 폴더가 Git 저장소인지 확인하세요.",
        ),
        (
            ErrorCode::HarnessNotInstalled,
            "The machine could not complete this Git action.",
            "Read the details below, correct the problem, and try again.",
            "머신이 이 Git 동작을 완료하지 못했습니다.",
            "아래 상세 출력을 읽고 문제를 고친 뒤 다시 시도하세요.",
        ),
        (
            ErrorCode::SpawnFailed,
            "The machine could not complete this Git action.",
            "Read the details below, correct the problem, and try again.",
            "머신이 이 Git 동작을 완료하지 못했습니다.",
            "아래 상세 출력을 읽고 문제를 고친 뒤 다시 시도하세요.",
        ),
        (
            ErrorCode::Unavailable,
            "The repository or machine is busy.",
            "Wait for the other operation to finish, then try again.",
            "저장소나 머신이 다른 작업 중입니다.",
            "다른 동작이 끝날 때까지 기다린 뒤 다시 시도하세요.",
        ),
        (
            ErrorCode::Internal,
            "The machine could not complete this Git action.",
            "Read the details below, correct the problem, and try again.",
            "머신이 이 Git 동작을 완료하지 못했습니다.",
            "아래 상세 출력을 읽고 문제를 고친 뒤 다시 시도하세요.",
        ),
    ];
    for (code, title_en, next_en, title_ko, next_ko) in cases {
        let problem = Problem::Node {
            code,
            message: "exact diagnostic\nsecond line".to_owned(),
        };
        for (lang, title, next) in [(Lang::En, title_en, next_en), (Lang::Ko, title_ko, next_ko)] {
            assert_eq!(
                lang.git_call_failure(&problem),
                GitFailureText {
                    title: title.to_owned(),
                    next: next.to_owned(),
                    detail: Some("exact diagnostic\nsecond line".to_owned()),
                }
            );
        }
    }
}

#[test]
fn older_and_disconnected_calls_are_worded_without_display_text() {
    for (lang, title, next) in [
        (
            Lang::En,
            "The app could not complete this request to the machine.",
            "Reconnect the machine, then try again.",
        ),
        (
            Lang::Ko,
            "이 머신에 대한 요청을 완료하지 못했습니다.",
            "머신을 다시 연결한 뒤 시도하세요.",
        ),
    ] {
        assert_eq!(
            lang.git_call_failure(&Problem::Gone("transport diagnostic".to_owned())),
            GitFailureText {
                title: title.to_owned(),
                next: next.to_owned(),
                detail: Some("transport diagnostic".to_owned()),
            }
        );
        let older = lang.git_call_failure(&Problem::Older);
        let node = lang.git_call_failure(&Problem::Node {
            code: ErrorCode::ProtocolMismatch,
            message: String::new(),
        });
        assert_eq!(
            (older.title, older.next, older.detail),
            (node.title, node.next, None)
        );
    }
}

#[test]
fn a_call_the_app_never_sent_says_so_without_node_detail_or_reconnect_advice() {
    assert_eq!(
        Lang::En.git_call_failure(&Problem::Unsent),
        GitFailureText {
            title: "Could not send.".to_owned(),
            next: "Try again.".to_owned(),
            detail: None,
        }
    );
    assert_eq!(
        Lang::Ko.git_call_failure(&Problem::Unsent),
        GitFailureText {
            title: "보내지 못했습니다.".to_owned(),
            next: "다시 시도하세요.".to_owned(),
            detail: None,
        }
    );
}
