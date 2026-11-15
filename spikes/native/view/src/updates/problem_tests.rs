//! Exact recovery sentences for every classified update problem.

use super::{BuildName, PreparedUpdate, Problem, Status};
use crate::localization::{ENGLISH, Locale, Translator};

const SENTENCES: &[(Problem, &str, &str)] = &[
    (
        Problem::Check,
        "The update check could not be completed. Try again later.",
        "업데이트 확인을 마치지 못했습니다. 나중에 다시 시도하세요.",
    ),
    (
        Problem::FeedTooLarge,
        "The update information is too large for this app. Try again later.",
        "업데이트 정보가 이 앱이 읽을 수 있는 크기보다 큽니다. 나중에 다시 시도하세요.",
    ),
    (
        Problem::FeedInvalid,
        "The update information could not be read. Try again later.",
        "업데이트 정보를 읽지 못했습니다. 나중에 다시 시도하세요.",
    ),
    (
        Problem::FeedUnverified,
        "The update information could not be verified. This app was not changed.",
        "업데이트 정보를 검증하지 못했습니다. 이 앱은 변경되지 않았습니다.",
    ),
    (
        Problem::FeedNewFormat,
        "This app cannot read the newer update format. Install the latest build manually.",
        "이 앱은 새로운 업데이트 형식을 읽을 수 없습니다. 최신 빌드를 직접 설치하세요.",
    ),
    (
        Problem::NoBuild,
        "This update has no build for this device.",
        "이 기기에 맞는 업데이트 빌드가 없습니다.",
    ),
    (
        Problem::Download,
        "The update could not be downloaded. Check your connection and try again.",
        "업데이트를 다운로드하지 못했습니다. 연결을 확인하고 다시 시도하세요.",
    ),
    (
        Problem::DownloadWrite,
        "The update could not be saved beside this app. Check the app folder's write permissions and try again.",
        "이 앱 옆에 업데이트를 저장하지 못했습니다. 앱 폴더에 쓸 수 있는지 확인하고 다시 시도하세요.",
    ),
    (
        Problem::DownloadIntegrity,
        "The download could not be verified. It was not installed. Try again.",
        "다운로드한 파일을 검증하지 못해 설치하지 않았습니다. 다시 시도하세요.",
    ),
    (
        Problem::InstallWrite,
        "The update could not replace this app. Check the app folder's write permissions and try again.",
        "업데이트로 이 앱을 교체하지 못했습니다. 앱 폴더에 쓸 수 있는지 확인하고 다시 시도하세요.",
    ),
    (
        Problem::Unsupported,
        "This build cannot update itself. Install the latest build manually.",
        "이 빌드는 스스로 업데이트할 수 없습니다. 최신 빌드를 직접 설치하세요.",
    ),
    (
        Problem::Interrupted,
        "The update was interrupted. Try again.",
        "업데이트가 중단되었습니다. 다시 시도하세요.",
    ),
    (
        Problem::Restart,
        "The updated app could not be started. Vessel is still open; try restarting it.",
        "업데이트된 앱을 시작하지 못했습니다. Vessel은 계속 열려 있으니 다시 시작해 보세요.",
    ),
];

#[test]
fn every_update_problem_is_a_whole_sentence_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let build = BuildName::from("dev");
    for &(problem, english, korean) in SENTENCES {
        for (catalog, expected) in [(en, english), (ko, korean)] {
            assert_eq!(problem.words(catalog), expected);
            let prepared =
                PreparedUpdate::new(catalog, &build, &Status::Failed { problem }, None).unwrap();
            assert_eq!(prepared.problem().as_deref(), Some(expected));
        }
    }
}

#[test]
fn a_checking_status_has_no_recovery_sentence() {
    let catalog = Translator::get(ENGLISH).unwrap();
    let build = BuildName::from("dev");
    let retrying = PreparedUpdate::new(catalog, &build, &Status::Checking, None).unwrap();
    assert!(retrying.problem().is_none());
}
