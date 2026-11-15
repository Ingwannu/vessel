use super::*;
use crate::localization::{ENGLISH, Locale};

#[test]
fn every_link_variant_has_whole_sentences_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    for (error, english, korean) in [
        (
            vessel_proto::LinkError::Prefix,
            "This is not a pairing link. Copy a pairing link from your machine and paste it here.",
            "페어링 링크가 아닙니다. 머신에서 페어링 링크를 복사해 붙여넣으세요.",
        ),
        (
            vessel_proto::LinkError::Encoding,
            "This pairing link is damaged. Copy the complete pairing link from your machine again.",
            "페어링 링크가 손상되었습니다. 머신에서 페어링 링크 전체를 다시 복사하세요.",
        ),
        (
            vessel_proto::LinkError::Offer { column: 17 },
            "This pairing link could not be read. Create a new pairing link on your machine and try again.",
            "페어링 링크를 읽을 수 없습니다. 머신에서 새 페어링 링크를 만든 뒤 다시 시도하세요.",
        ),
    ] {
        let problem = LinkProblem::from(&error);
        assert_eq!(problem.words(en), english);
        assert_eq!(problem.words(ko), korean);
    }
}

#[test]
fn every_name_problem_has_whole_sentences_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    for (problem, english, korean) in [
        (
            NameProblem::Empty,
            "Type a short name, for example home-pc.",
            "짧은 이름을 적으세요 (예: home-pc).",
        ),
        (
            NameProblem::LooksLikeKey,
            "That is the key. Type a short name instead, for example home-pc.",
            "이것은 키입니다. 대신 짧은 이름을 적으세요 (예: home-pc).",
        ),
        (
            NameProblem::LooksLikeLink,
            "That is the pairing link. Type a short name here, or leave it empty to use the machine's own name.",
            "이것은 페어링 링크입니다. 여기에는 짧은 이름을 적거나, 머신 자신의 이름을 쓰려면 비워 두세요.",
        ),
        (
            NameProblem::Invalid,
            "Use letters, digits, '.', '_' and '-', starting with a letter or digit.",
            "영문자, 숫자, '.', '_', '-'만 쓸 수 있고 영문자나 숫자로 시작해야 합니다.",
        ),
        (
            NameProblem::Taken,
            "Another machine already has this name.",
            "이미 같은 이름의 머신이 있습니다.",
        ),
    ] {
        assert_eq!(problem.words(en), english);
        assert_eq!(problem.words(ko), korean);
    }
}
#[test]
fn both_dialogs_keep_every_whole_sentence_in_both_languages() {
    for (locale, expected) in [
        (
            ENGLISH,
            [
                "Paste pairing link",
                "Paste the pairing link your machine printed.",
                "Pairing link",
                "Treat this pairing link like a password. Anyone with it can access this machine.",
                "Cancel",
                "Pair",
                "Pairing...",
                "Connect to this machine?",
                "This machine will be able to run code in this app and reach your other connected machines. Only connect if you recognize it.",
                "Machine",
                "Key fingerprint",
                "Relay",
                "Connect",
            ],
        ),
        (
            Locale::from_tag("ko").unwrap(),
            [
                "페어링 링크 붙여넣기",
                "머신이 출력한 페어링 링크를 붙여넣으세요.",
                "페어링 링크",
                "이 페어링 링크는 비밀번호처럼 취급하세요. 링크를 가진 사람은 누구나 이 머신에 접근할 수 있습니다.",
                "취소",
                "페어링",
                "페어링 중...",
                "이 머신에 연결할까요?",
                "이 머신은 이 앱에서 코드를 실행하고 연결된 다른 머신에 접근할 수 있게 됩니다. 알고 있는 머신인 경우에만 연결하세요.",
                "머신",
                "키 지문",
                "릴레이",
                "연결",
            ],
        ),
    ] {
        let catalog = Translator::get(locale).unwrap();
        let words = catalog.pairing();
        assert_eq!(
            [
                words.title(),
                words.helper(),
                words.label(),
                words.security_warning(),
                words.cancel(),
                words.pair(),
                words.busy(),
                words.confirm_title(),
                words.confirm_description(),
                words.machine(),
                words.fingerprint(),
                words.relay(),
                words.connect(),
            ],
            expected
        );
    }
}

#[test]
fn every_pair_failure_has_a_whole_recovery_message_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    for (problem, english, korean) in [
        (
            PairProblem::Name,
            "This name can no longer be used. Choose another name and try again.",
            "이 이름을 더 이상 사용할 수 없습니다. 다른 이름으로 다시 시도하세요.",
        ),
        (
            PairProblem::Storage,
            "Could not save this machine. Check that the app can write to its configuration folder, then create a new pairing link and try again.",
            "머신을 저장할 수 없습니다. 앱이 설정 폴더에 쓸 수 있는지 확인한 뒤, 새 페어링 링크를 만들어 다시 시도하세요.",
        ),
        (
            PairProblem::Connection,
            "Could not connect to this machine. Check that it is running, then create a new pairing link and try again.",
            "머신에 연결할 수 없습니다. 머신이 실행 중인지 확인한 뒤, 새 페어링 링크를 만들어 다시 시도하세요.",
        ),
        (
            PairProblem::Browser,
            "Pairing is unavailable in this browser. Pair this machine from the desktop app.",
            "이 브라우저에서는 페어링할 수 없습니다. 데스크톱 앱에서 이 머신을 페어링하세요.",
        ),
    ] {
        assert_eq!(problem.words(en), english);
        assert_eq!(problem.words(ko), korean);
    }
    assert_eq!(
        LinkProblem::Missing.words(en),
        "Paste a pairing link first."
    );
    assert_eq!(
        LinkProblem::Missing.words(ko),
        "먼저 페어링 링크를 붙여 넣으세요."
    );
}
#[test]
fn visible_failures_change_language_without_losing_the_typed_cause() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let mut failure = PreparedFailure::new(PairProblem::Connection, en);
    assert_eq!(
        failure.words(),
        "Could not connect to this machine. Check that it is running, then create a new pairing link and try again."
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let allocations = allocation_counter::measure(|| {
            for _ in 0..10_000 {
                let _words = std::hint::black_box(failure.words());
                let _shared = std::hint::black_box(failure.shared_words());
            }
        });
        assert_eq!(allocations.count_total, 0);
        assert_eq!(allocations.bytes_total, 0);
    }
    failure.translate(ko);
    assert_eq!(
        failure.words(),
        "머신에 연결할 수 없습니다. 머신이 실행 중인지 확인한 뒤, 새 페어링 링크를 만들어 다시 시도하세요."
    );
    assert_eq!(failure.problem, PairProblem::Connection);

    let fault = crate::files::FileFault {
        path: "nodes.toml".to_owned(),
        at: Some((3, 9)),
        problem: crate::files::FileProblem::WrongKind,
        field: Some("token".to_owned()),
    };
    let mut failure = PreparedFailure::new(PairProblem::Book(fault.clone()), en);
    assert_eq!(
        failure.words(),
        "The machine list nodes.toml does not read (line 3, column 9: `token` names the file that holds the token, token = { file = \"…\" }, never the token itself). Fix it, then reload."
    );
    failure.translate(ko);
    assert_eq!(
        failure.words(),
        "머신 목록 파일을 읽지 못했습니다: nodes.toml (3번째 줄 9번째 칸: `token`에는 토큰 자체가 아니라 토큰이 든 파일을 적습니다: token = { file = \"…\" }). 고친 뒤 새로고침하세요."
    );
    assert_eq!(failure.problem, PairProblem::Book(fault));
}

#[test]
fn successful_notice_keeps_the_whole_sentence_and_changes_without_repairing() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    for (via, english, korean) in [
        (
            crate::Via::Direct,
            "Paired with \u{2068}home-pc\u{2069}, connected directly",
            "\u{2068}home-pc\u{2069} 머신에 직접 연결됨",
        ),
        (
            crate::Via::Relay,
            "Paired with \u{2068}home-pc\u{2069}, connected through a relay",
            "\u{2068}home-pc\u{2069} 머신에 릴레이로 연결됨",
        ),
    ] {
        let mut notice = PairedNotice::new("home-pc", via, en).unwrap();
        assert_eq!(notice.words(), english);
        notice.translate(ko).unwrap();
        assert_eq!(notice.words(), korean);
        assert_eq!(notice.name, "home-pc");
        assert_eq!(notice.via, via);
    }
}

#[test]
fn fields_keep_their_whole_context_words_after_leaving_the_mixed_table() {
    for (locale, expected) in [
        (
            ENGLISH,
            [
                "vessel://pair#…",
                "Name for this machine",
                "A short name you will recognise, for example home-pc.",
                "Leave empty to use its own name",
                "Paste a pairing link first.",
            ],
        ),
        (
            Locale::from_tag("ko").unwrap(),
            [
                "vessel://pair#…",
                "이 머신의 이름",
                "알아보기 쉬운 짧은 이름 (예: home-pc)",
                "비워 두면 머신 자신의 이름을 씁니다",
                "먼저 페어링 링크를 붙여 넣으세요.",
            ],
        ),
    ] {
        let catalog = Translator::get(locale).unwrap();
        let words = catalog.pairing();
        assert_eq!(
            [
                words.link_placeholder(),
                words.name_label(),
                words.name_hint(),
                words.name_placeholder(),
                words.paste_first(),
            ],
            expected
        );
    }
}
