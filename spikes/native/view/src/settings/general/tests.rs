//! Whole sentences keep a translation from becoming a collection of fragments.
use super::StorageError;
use crate::files::{FileFault, FileProblem};
use crate::localization::{ENGLISH, Locale, Translator};
use crate::prefs::{Kept, LaunchOnly};

fn catalog(tag: &str) -> &'static Translator {
    Translator::get(Locale::from_tag(tag).unwrap()).unwrap()
}

fn fault(problem: FileProblem, field: Option<&str>, at: Option<(usize, usize)>) -> Kept {
    Kept::ThisLaunch {
        why: LaunchOnly::Unreadable {
            fault: FileFault {
                path: "prefs.json".to_owned(),
                at,
                problem,
                field: field.map(str::to_owned),
            },
        },
    }
}

#[test]
fn file_and_launch_only_explanations_are_whole_messages() {
    let file = Kept::File {
        path: "prefs.json".to_owned(),
    };
    let absent = Kept::default();
    let path = Kept::ThisLaunch {
        why: LaunchOnly::NoPath {
            message: "arbitrary private diagnostic".to_owned(),
        },
    };
    let newer = Kept::ThisLaunch {
        why: LaunchOnly::Newer {
            path: "설정".to_owned(),
            version: 1001,
        },
    };
    let en = catalog("en").general();
    let ko = catalog("ko").general();
    assert_eq!(
        en.storage(&file).unwrap(),
        "Kept by Vessel on this computer, so every launch opens with them.\n\u{2068}prefs.json\u{2069}"
    );
    assert_eq!(
        ko.storage(&file).unwrap(),
        "Vessel이 이 컴퓨터에 보관하므로, 실행할 때마다 그대로 열립니다.\n\u{2068}prefs.json\u{2069}"
    );
    assert_eq!(
        en.storage(&absent).unwrap(),
        "Kept until Vessel quits: this computer gives Vessel no place to keep them."
    );
    assert_eq!(
        ko.storage(&absent).unwrap(),
        "Vessel을 끝낼 때까지만 보관합니다: 이 컴퓨터에 보관할 곳이 없습니다."
    );
    assert_eq!(
        en.storage(&path).unwrap(),
        "Kept until Vessel quits: the preferences location could not be determined."
    );
    assert_eq!(
        ko.storage(&path).unwrap(),
        "Vessel을 끝낼 때까지만 보관합니다: 환경설정 위치를 정할 수 없습니다."
    );
    assert_eq!(
        en.storage(&newer).unwrap(),
        "Kept until Vessel quits: \u{2068}설정\u{2069} was written by a newer Vessel (version \u{2068}1001\u{2069}); it is left as it is."
    );
    assert_eq!(
        ko.storage(&newer).unwrap(),
        "Vessel을 끝낼 때까지만 보관합니다: \u{2068}설정\u{2069}\u{2068}은\u{2069} 더 새 Vessel(버전 \u{2068}1001\u{2069})이 썼습니다. 그대로 둡니다."
    );
}

#[test]
fn every_problem_has_a_whole_english_recovery() {
    let words = catalog("en").general();
    for (problem, field, expected) in [
        (
            FileProblem::NotToml,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (\u{2068}this is not TOML (look for an unclosed quote or bracket, or a line without `=`)\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::NotJson,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (\u{2068}this is not JSON\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::UnknownKey,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (\u{2068}a key this file does not take\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::DuplicateKey,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (\u{2068}a key given twice\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::MissingKey,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (\u{2068}a key this file needs is missing\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::WrongKind,
            Some("token"),
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (\u{2068}`token` names the file that holds the token, token = { file = \"…\" }, never the token itself\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::WrongKind,
            Some("settings"),
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (`\u{2068}settings\u{2069}` has a value of the wrong kind). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::WrongKind,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (\u{2068}a value of the wrong kind\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::CannotOpen,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (\u{2068}it cannot be opened (check who owns it and its mode)\u{2069}). Fix or remove it, then restart Vessel.",
        ),
    ] {
        assert_eq!(
            words.storage(&fault(problem, field, None)).unwrap(),
            expected
        );
    }
}

#[test]
fn every_problem_has_a_whole_korean_recovery() {
    let words = catalog("ko").general();
    for (problem, field, expected) in [
        (
            FileProblem::NotToml,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}TOML 형식이 아닙니다(닫히지 않은 따옴표나 괄호, `=`가 없는 줄을 확인하세요)\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::NotJson,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}JSON 형식이 아닙니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::UnknownKey,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}이 파일에서 쓰지 않는 키가 있습니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::DuplicateKey,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}같은 키가 두 번 있습니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::MissingKey,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}꼭 있어야 하는 키가 없습니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::WrongKind,
            Some("token"),
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}`token`에는 토큰 자체가 아니라 토큰이 든 파일을 적습니다: token = { file = \"…\" }\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::WrongKind,
            Some("settings"),
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (`\u{2068}settings\u{2069}` 값의 종류가 맞지 않습니다). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::WrongKind,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}값의 종류가 맞지 않습니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::CannotOpen,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}파일을 열 수 없습니다(소유자와 권한을 확인하세요)\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
    ] {
        assert_eq!(
            words.storage(&fault(problem, field, None)).unwrap(),
            expected
        );
    }
}

#[test]
fn located_and_named_faults_keep_real_positions_without_grouping() {
    let known = fault(FileProblem::WrongKind, Some("settings"), Some((1001, 23)));
    let syntax = fault(FileProblem::NotJson, None, Some((12, 3)));
    assert_eq!(
        catalog("en").general().storage(&known).unwrap(),
        "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}1001\u{2069}, column \u{2068}23\u{2069}: `\u{2068}settings\u{2069}` has a value of the wrong kind). Fix or remove it, then restart Vessel."
    );
    assert_eq!(
        catalog("ko").general().storage(&known).unwrap(),
        "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}1001\u{2069}번째 줄 \u{2068}23\u{2069}번째 칸: `\u{2068}settings\u{2069}` 값의 종류가 맞지 않습니다). 고치거나 지운 뒤 Vessel을 다시 시작하세요."
    );
    assert_eq!(
        catalog("en").general().storage(&syntax).unwrap(),
        "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: \u{2068}this is not JSON\u{2069}). Fix or remove it, then restart Vessel."
    );
    assert_eq!(
        catalog("ko").general().storage(&syntax).unwrap(),
        "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: \u{2068}JSON 형식이 아닙니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요."
    );
}

#[test]
fn invalid_position_is_rejected_instead_of_silently_truncated() {
    if let Ok(max) = usize::try_from(u32::MAX)
        && let Some(line) = max.checked_add(1)
    {
        let kept = fault(FileProblem::NotJson, None, Some((line, 1)));
        assert!(matches!(
            Translator::get(ENGLISH).unwrap().general().storage(&kept),
            Err(StorageError::Position(_))
        ));
    }
}

#[test]
fn pseudo_storage_keeps_paths_and_selected_recovery() {
    let kept = fault(FileProblem::NotJson, None, Some((12, 3)));
    let pseudo = catalog("en-XA").general().storage(&kept).unwrap();
    assert_ne!(pseudo, catalog("en").general().storage(&kept).unwrap());
    assert!(pseudo.contains("prefs.json"));
    assert!(!pseudo.contains("NUMBER()"));
}

#[test]
fn located_problems_have_whole_english_recoveries() {
    let words = catalog("en").general();
    for (problem, field, expected) in [
        (
            FileProblem::NotToml,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: \u{2068}this is not TOML (look for an unclosed quote or bracket, or a line without `=`)\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::NotJson,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: \u{2068}this is not JSON\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::UnknownKey,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: \u{2068}a key this file does not take\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::DuplicateKey,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: \u{2068}a key given twice\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::MissingKey,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: \u{2068}a key this file needs is missing\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::WrongKind,
            Some("token"),
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: \u{2068}`token` names the file that holds the token, token = { file = \"…\" }, never the token itself\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::WrongKind,
            Some("settings"),
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: `\u{2068}settings\u{2069}` has a value of the wrong kind). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::WrongKind,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: \u{2068}a value of the wrong kind\u{2069}). Fix or remove it, then restart Vessel.",
        ),
        (
            FileProblem::CannotOpen,
            None,
            "Kept until Vessel quits: \u{2068}prefs.json\u{2069} could not be read (line \u{2068}12\u{2069}, column \u{2068}3\u{2069}: \u{2068}it cannot be opened (check who owns it and its mode)\u{2069}). Fix or remove it, then restart Vessel.",
        ),
    ] {
        assert_eq!(
            words
                .storage(&fault(problem, field, Some((12, 3))))
                .unwrap(),
            expected
        );
    }
}

#[test]
fn located_problems_have_whole_korean_recoveries() {
    let words = catalog("ko").general();
    for (problem, field, expected) in [
        (
            FileProblem::NotToml,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: \u{2068}TOML 형식이 아닙니다(닫히지 않은 따옴표나 괄호, `=`가 없는 줄을 확인하세요)\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::NotJson,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: \u{2068}JSON 형식이 아닙니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::UnknownKey,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: \u{2068}이 파일에서 쓰지 않는 키가 있습니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::DuplicateKey,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: \u{2068}같은 키가 두 번 있습니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::MissingKey,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: \u{2068}꼭 있어야 하는 키가 없습니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::WrongKind,
            Some("token"),
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: \u{2068}`token`에는 토큰 자체가 아니라 토큰이 든 파일을 적습니다: token = { file = \"…\" }\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::WrongKind,
            Some("settings"),
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: `\u{2068}settings\u{2069}` 값의 종류가 맞지 않습니다). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::WrongKind,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: \u{2068}값의 종류가 맞지 않습니다\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
        (
            FileProblem::CannotOpen,
            None,
            "Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: \u{2068}prefs.json\u{2069} (\u{2068}12\u{2069}번째 줄 \u{2068}3\u{2069}번째 칸: \u{2068}파일을 열 수 없습니다(소유자와 권한을 확인하세요)\u{2069}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
        ),
    ] {
        assert_eq!(
            words
                .storage(&fault(problem, field, Some((12, 3))))
                .unwrap(),
            expected
        );
    }
}
