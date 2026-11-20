//! Paseo's checks words, word for word in English, whole in Korean.

use vessel_proto::forge::{CheckRun, CheckState};

use crate::localization::{ENGLISH, Locale, Translator};
use crate::workspace::pull::checks::{Presentation, Timing, summary};

fn en() -> &'static Translator {
    Translator::get(ENGLISH).unwrap()
}

fn ko() -> &'static Translator {
    Translator::get(Locale::from_tag("ko").unwrap()).unwrap()
}

/// Without Fluent's isolation marks around values.
fn plain(text: &str) -> String {
    text.replace(['\u{2068}', '\u{2069}'], "")
}

fn run(name: &str, state: CheckState) -> CheckRun {
    CheckRun::named(name.parse().unwrap(), state, None)
}

#[test]
fn the_detail_line_and_groups_read_as_paseo_s() {
    let runs = [
        run("a", CheckState::Failed),
        run("b", CheckState::Failed),
        run("c", CheckState::Pending),
        run("d", CheckState::Passed),
    ];
    let read = summary(&runs);
    assert_eq!(
        plain(&read.detail_words(en())),
        "2 failing, 1 in progress, 1 successful checks"
    );
    assert_eq!(
        plain(&summary(&runs[3..]).detail_words(en())),
        "1 successful check"
    );
    assert_eq!(summary(&[]).detail_words(en()), "", "no checks, no line");
    assert_eq!(
        plain(&Presentation::Failure.group_words(2, en())),
        "2 failing checks"
    );
    assert_eq!(
        plain(&Presentation::Ignored.group_words(1, en())),
        "1 skipped check"
    );
    assert_eq!(
        plain(&Presentation::Pending.group_words(1, en())),
        "1 in progress check"
    );
    assert_eq!(
        plain(&Presentation::Success.group_words(3, ko())),
        "성공한 검사 3개"
    );
    assert_eq!(
        plain(&read.detail_words(ko())),
        "검사 4개: 2개 실패, 1개 진행 중, 1개 성공"
    );
}

#[test]
fn a_running_check_says_so() {
    let done = Timing {
        running: false,
        seconds: 90,
    };
    assert_eq!(done.words(en()), "1m 30s");
    let running = Timing {
        running: true,
        seconds: 120,
    };
    assert_eq!(plain(&running.words(en())), "running 2m");
    assert_eq!(plain(&running.words(ko())), "2m 실행 중");
}
