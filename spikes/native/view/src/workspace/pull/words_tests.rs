//! Paseo's sentences in both languages, a host kept as a value with its
//! Korean particle, and the forge's refusal never pasted in as a sentence.

use vessel_proto::forge::{ForgeAccess, ForgeHost, GhState, OpenRefusal};

use super::refusal_words;
use crate::localization::{ENGLISH, Locale, Translator};
use crate::workspace::pull::{CheckCount, ChecksOutcome, PullPhase, Unavailable};

fn en() -> &'static Translator {
    Translator::get(ENGLISH).unwrap()
}

fn ko() -> &'static Translator {
    Translator::get(Locale::from_tag("ko").unwrap()).unwrap()
}

fn github() -> ForgeHost {
    "github.com".parse().unwrap()
}

/// Without Fluent's isolation marks around values.
fn plain(text: &str) -> String {
    text.replace(['\u{2068}', '\u{2069}'], "")
}

#[test]
fn paseo_s_two_sentences_are_kept_word_for_word() {
    assert_eq!(
        Unavailable::NoCommits.words(en()),
        "Create PR isn't available because this branch doesn't have any new commits yet"
    );
    assert_eq!(
        Unavailable::NoCommits.words(ko()),
        "이 브랜치에 아직 새 커밋이 없어 PR 생성을 사용할 수 없습니다"
    );
    assert_eq!(
        plain(&Unavailable::NotSupported(github()).words(en())),
        "Create PR isn't available right now because github.com isn't connected"
    );
}

#[test]
fn a_host_takes_the_korean_particle_its_last_sound_asks_for() {
    let words = plain(&Unavailable::SignedOut(github()).words(ko()));
    assert!(
        words.starts_with("이 머신이 github.com에 로그인"),
        "{words}"
    );
    let refused = plain(&Unavailable::Refused(github()).words(ko()));
    assert!(refused.starts_with("github.com이 "), "{refused}");
}

#[test]
fn a_refusal_names_its_family_and_leaves_the_forge_s_message_to_the_detail() {
    let message = "Validation Failed: a pull request already exists".to_owned();
    let rejected = refusal_words(
        &OpenRefusal::Rejected {
            message: message.clone(),
        },
        en(),
    );
    assert_eq!(rejected, "The pull request was refused");
    assert!(!rejected.contains(&message));
    let signed_out = OpenRefusal::Access {
        access: ForgeAccess::SignedOut {
            host: github(),
            gh: GhState::SignedOut,
        },
    };
    assert!(plain(&refusal_words(&signed_out, en())).contains("isn't signed in to github.com"));
    assert_eq!(
        refusal_words(&OpenRefusal::Unknown, ko()),
        "풀 리퀘스트를 생성하지 못했습니다"
    );
}

#[test]
fn the_facts_read_as_paseo_s_pane_reads_them() {
    assert_eq!(PullPhase::Draft.words(ko()), "초안");
    assert_eq!(
        ChecksOutcome::Failure.words(en()),
        "Some checks were not successful"
    );
    assert_eq!(plain(&CheckCount::Failing(2).words(en())), "2 failing");
    assert_eq!(plain(&CheckCount::Passed(5).words(ko())), "5개 성공");
}
