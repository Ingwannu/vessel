//! Whole-message parity with the retired update table, plus cached values.

use super::*;
use crate::localization::{ENGLISH, LanguagePreference, PSEUDO, resolve_language};

fn catalogs() -> (&'static Translator, &'static Translator) {
    (
        Translator::get(ENGLISH).unwrap(),
        Translator::get(resolve_language(
            LanguagePreference::System,
            Some("ko"),
            None,
        ))
        .unwrap(),
    )
}

#[test]
fn plain_sentences_1_keep_the_table_words() {
    let (en, ko) = catalogs();
    assert_eq!(en.update().checking(), "Checking for updates…");
    assert_eq!(ko.update().checking(), "업데이트 확인 중…");
    assert_eq!(en.update().restart(), "Restart to update");
    assert_eq!(ko.update().restart(), "다시 시작해 업데이트");
    assert_eq!(en.update().failed(), "Update failed");
    assert_eq!(ko.update().failed(), "업데이트하지 못했습니다");
    assert_eq!(en.update().try_again(), "Try again");
    assert_eq!(ko.update().try_again(), "다시 시도");
    assert_eq!(en.update().about(), "About");
    assert_eq!(ko.update().about(), "정보");
    assert_eq!(en.update().app_version(), "App version");
    assert_eq!(ko.update().app_version(), "앱 버전");
    assert_eq!(en.update().this_device(), "This device");
    assert_eq!(ko.update().this_device(), "이 기기");
    assert_eq!(en.update().app_updates(), "App updates");
    assert_eq!(ko.update().app_updates(), "앱 업데이트");
    assert_eq!(en.update().check(), "Check");
    assert_eq!(ko.update().check(), "확인");
    assert_eq!(en.update().checking_short(), "Checking...");
    assert_eq!(ko.update().checking_short(), "확인 중...");
}

#[test]
fn plain_sentences_2_keep_the_table_words() {
    let (en, ko) = catalogs();
    assert_eq!(en.update().update(), "Update");
    assert_eq!(ko.update().update(), "업데이트");
    assert_eq!(en.update().status_checking(), "Checking for app updates...");
    assert_eq!(ko.update().status_checking(), "앱 업데이트 확인 중...");
    assert_eq!(en.update().status_up_to_date(), "App is up to date.");
    assert_eq!(ko.update().status_up_to_date(), "앱이 최신 상태입니다.");
    assert_eq!(en.update().status_failed(), "Failed to update app.");
    assert_eq!(ko.update().status_failed(), "앱을 업데이트하지 못했습니다.");
    assert_eq!(
        en.update().status_idle(),
        "Update status has not been checked yet."
    );
    assert_eq!(
        ko.update().status_idle(),
        "업데이트 상태를 아직 확인하지 않았습니다."
    );
    assert_eq!(en.update().log_file(), "Log file");
    assert_eq!(ko.update().log_file(), "로그 파일");
    assert_eq!(en.update().open_logs(), "Open logs");
    assert_eq!(ko.update().open_logs(), "로그 열기");
    assert_eq!(en.update().copy_path(), "Copy path");
    assert_eq!(ko.update().copy_path(), "경로 복사");
    assert_eq!(en.update().log_path_copied(), "Log path copied.");
    assert_eq!(ko.update().log_path_copied(), "로그 경로가 복사되었습니다.");
    assert_eq!(en.update().log_unavailable(), "Log path unavailable");
    assert_eq!(
        ko.update().log_unavailable(),
        "로그 경로를 사용할 수 없습니다"
    );
}

#[test]
fn plain_sentences_3_keep_the_table_words() {
    let (en, ko) = catalogs();
    assert_eq!(en.update().whats_new(), "What's new");
    assert_eq!(ko.update().whats_new(), "새로운 소식");
    assert_eq!(
        en.update().whats_new_hint(),
        "Release notes for this version"
    );
    assert_eq!(ko.update().whats_new_hint(), "이 버전의 릴리스 노트");
    assert_eq!(en.update().install_title(), "Install desktop update");
    assert_eq!(ko.update().install_title(), "데스크톱 업데이트 설치");
    assert_eq!(
        en.update().install_message(),
        "This updates Vessel on this computer"
    );
    assert_eq!(
        ko.update().install_message(),
        "이 컴퓨터의 Vessel을 업데이트합니다"
    );
    assert_eq!(en.update().install_confirm(), "Install update");
    assert_eq!(ko.update().install_confirm(), "업데이트 설치");
    assert_eq!(en.update().cancel(), "Cancel");
    assert_eq!(ko.update().cancel(), "취소");
    assert_eq!(en.update().automatic(), "Update automatically");
    assert_eq!(ko.update().automatic(), "자동으로 업데이트");
    assert_eq!(
        en.update().automatic_note(),
        "When off, Vessel looks for a new build only when you check."
    );
    assert_eq!(
        ko.update().automatic_note(),
        "끄면 확인을 누를 때만 새 빌드를 찾습니다."
    );
    assert_eq!(
        en.update().no_updates(),
        "This build does not update itself (a development build)."
    );
    assert_eq!(
        ko.update().no_updates(),
        "이 빌드는 스스로 업데이트하지 않습니다(개발 빌드)."
    );
    assert_eq!(en.update().dismiss(), "Dismiss");
    assert_eq!(ko.update().dismiss(), "닫기");
}

#[test]
fn a_numbered_build_reads_as_a_version_and_a_developers_as_its_word() {
    let (en, ko) = catalogs();
    for catalog in [en, ko] {
        for (label, expected) in [("0.0.0.0.7", "v\u{2068}0.0.0.0.7\u{2069}"), ("dev", "dev")] {
            let ready =
                PreparedUpdate::new(catalog, &BuildName::from(label), &Status::Idle, None).unwrap();
            assert_eq!(&*ready.version(), expected);
        }
    }
}

#[test]
fn a_download_and_ready_build_are_whole_sentences_in_both_languages() {
    let (en, ko) = catalogs();
    let build = BuildName::from("0.0.0.0.7");
    let downloading = Status::Downloading {
        build: build.clone(),
        progress: Number::count(42),
    };
    let ready = Status::Ready {
        build: build.clone(),
    };
    for (catalog, status, action, progress, banner, ready_status, ready_action) in [
        (
            en,
            "Update found: v\u{2068}0.0.0.0.7\u{2069}. Downloading...",
            "Update",
            "Downloading update… \u{2068}42\u{2069}%",
            "Updated to \u{2068}0.0.0.0.7\u{2069}",
            "Update ready: v\u{2068}0.0.0.0.7\u{2069}",
            "Update to v\u{2068}0.0.0.0.7\u{2069}",
        ),
        (
            ko,
            "업데이트 발견: v\u{2068}0.0.0.0.7\u{2069}. 다운로드 중...",
            "업데이트",
            "업데이트 받는 중… \u{2068}42\u{2069}%",
            "업데이트를 마쳤습니다: \u{2068}0.0.0.0.7\u{2069}",
            "업데이트 준비됨: v\u{2068}0.0.0.0.7\u{2069}",
            "업데이트 설치: v\u{2068}0.0.0.0.7\u{2069}",
        ),
    ] {
        let first = PreparedUpdate::new(catalog, &build, &downloading, Some(&build)).unwrap();
        assert_eq!(&*first.status(), status);
        assert_eq!(&*first.action(), action);
        assert_eq!(first.progress().as_deref(), Some(progress));
        assert_eq!(first.banner().as_deref(), Some(banner));
        let second = PreparedUpdate::new(catalog, &build, &ready, None).unwrap();
        assert_eq!(&*second.status(), ready_status);
        assert_eq!(&*second.action(), ready_action);
        assert!(second.progress().is_none());
        assert!(second.banner().is_none());
    }
}

#[test]
fn other_statuses_and_preparation_failure_have_whole_sentences() {
    let (en, ko) = catalogs();
    for (state, english, korean) in [
        (
            Status::Idle,
            "Update status has not been checked yet.",
            "업데이트 상태를 아직 확인하지 않았습니다.",
        ),
        (
            Status::Checking,
            "Checking for app updates...",
            "앱 업데이트 확인 중...",
        ),
        (
            Status::Current,
            "App is up to date.",
            "앱이 최신 상태입니다.",
        ),
        (
            Status::Failed {
                problem: Problem::Check,
            },
            "Failed to update app.",
            "앱을 업데이트하지 못했습니다.",
        ),
    ] {
        for (catalog, expected, action) in [(en, english, "Update"), (ko, korean, "업데이트")] {
            let prepared =
                PreparedUpdate::new(catalog, &BuildName::from("dev"), &state, None).unwrap();
            assert_eq!(&*prepared.status(), expected);
            assert_eq!(&*prepared.action(), action);
        }
    }
    assert_eq!(
        en.update().details_unavailable(),
        "Update information is unavailable."
    );
    assert_eq!(
        ko.update().details_unavailable(),
        "업데이트 정보를 표시할 수 없습니다."
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn prepared_getters_allocate_nothing_for_a_frame() {
    const FRAME_PROBES: usize = 10_000;
    let (en, ko) = catalogs();
    for catalog in [en, ko, Translator::get(PSEUDO).unwrap()] {
        let build = BuildName::from("0.0.0.0.7");
        for state in [
            Status::Idle,
            Status::Failed {
                problem: Problem::Check,
            },
            Status::Ready {
                build: build.clone(),
            },
            Status::Downloading {
                build: build.clone(),
                progress: Number::count(42),
            },
        ] {
            let prepared = PreparedUpdate::new(catalog, &build, &state, Some(&build)).unwrap();
            let allocations = allocation_counter::measure(|| {
                for _ in 0..FRAME_PROBES {
                    let _version = std::hint::black_box(prepared.version());
                    let _status = std::hint::black_box(prepared.status());
                    let _action = std::hint::black_box(prepared.action());
                    let _progress = std::hint::black_box(prepared.progress());
                    let _banner = std::hint::black_box(prepared.banner());
                    let _problem = std::hint::black_box(prepared.problem());
                }
            });
            assert_eq!(allocations.count_total, 0);
            assert_eq!(allocations.bytes_total, 0);
        }
    }
}

#[test]
fn a_named_build_does_not_gain_a_numeric_version_prefix_in_sentences() {
    let (en, ko) = catalogs();
    let build = BuildName::from("dev");
    for (catalog, status, action) in [
        (
            en,
            "Update ready: \u{2068}dev\u{2069}",
            "Update to \u{2068}dev\u{2069}",
        ),
        (
            ko,
            "업데이트 준비됨: \u{2068}dev\u{2069}",
            "업데이트 설치: \u{2068}dev\u{2069}",
        ),
    ] {
        let prepared = PreparedUpdate::new(
            catalog,
            &build,
            &Status::Ready {
                build: build.clone(),
            },
            None,
        )
        .unwrap();
        assert_eq!(&*prepared.status(), status);
        assert_eq!(&*prepared.action(), action);
    }
}
