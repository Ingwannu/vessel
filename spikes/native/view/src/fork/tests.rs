//! Whole sentences, title updates, first-match semantics and retained caches.
use super::*;
use crate::localization::{ENGLISH, Locale};
use crate::{Card, Machine, Reach};
use vessel_proto::{HarnessKind, SessionInfo, SessionMode, SessionSpec};

fn catalog(tag: &str) -> &'static Translator {
    Translator::get(Locale::from_tag(tag).unwrap()).unwrap()
}

fn fleet() -> Fleet {
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let info = SessionInfo::new(
        SessionId::random(),
        spec,
        "2026-11-07T09:00:00Z".parse().unwrap(),
    );
    let mut card = Card::from_session("isolated", &info);
    card.title = " plain ".to_owned();
    Fleet {
        machines: vec![Machine {
            name: "isolated".to_owned(),
            reach: Reach::Online,
            sessions: vec![card],
            agents: Vec::new(),
            protocol: None,
            version: None,
        }],
        ..Fleet::default()
    }
}

#[test]
fn captions_follow_title_and_language_changes_and_evict_removed_sources() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = catalog("ko");
    let mut fleet = fleet();
    let source = fleet.machines[0].sessions[0].session;
    let mut headers = Headers::default();
    assert!(headers.refresh(&fleet, en).unwrap());
    assert_eq!(
        headers.words(source, en),
        "Forked from “\u{2068}plain\u{2069}”"
    );
    let retained = Arc::clone(&headers.captions[&source].words);
    assert!(!headers.refresh(&fleet, en).unwrap());
    assert!(Arc::ptr_eq(&retained, &headers.captions[&source].words));
    assert!(headers.refresh(&fleet, ko).unwrap());
    assert_eq!(
        headers.words(source, ko),
        "“\u{2068}plain\u{2069}”에서 분기함"
    );
    fleet.machines[0].sessions[0].title = "renamed".to_owned();
    assert!(headers.refresh(&fleet, ko).unwrap());
    assert_eq!(
        headers.words(source, ko),
        "“\u{2068}renamed\u{2069}”에서 분기함"
    );
    let mut duplicate = fleet.machines[0].sessions[0].clone();
    duplicate.title = "second match".to_owned();
    fleet.machines[0].sessions.push(duplicate);
    assert!(!headers.refresh(&fleet, ko).unwrap());
    fleet.machines[0].sessions[0].title = " ".to_owned();
    assert!(headers.refresh(&fleet, ko).unwrap());
    assert_eq!(headers.words(source, ko), "다른 세션에서 분기함");
    fleet.machines.clear();
    assert!(headers.refresh(&fleet, ko).unwrap());
    assert!(headers.captions.is_empty());
    assert_eq!(headers.words(source, en), "Forked from another session");
}

#[test]
fn plain_labels_and_staged_notices_match_the_existing_sentences() {
    let en = catalog("en");
    let ko = catalog("ko");
    assert_eq!(en.fork().tooltip(), "Fork chat from here");
    assert_eq!(ko.fork().tooltip(), "여기에서 채팅 분기");
    assert_eq!(en.fork().in_new_session(), "Fork in a new session");
    assert_eq!(ko.fork().in_new_session(), "새 세션으로 분기");
    assert_eq!(en.fork().empty_heading(), "The fork did not start");
    assert_eq!(ko.fork().empty_heading(), "분기가 시작되지 않았습니다");
    assert_eq!(en.fork().remove(), "Remove");
    assert_eq!(ko.fork().remove(), "삭제");
    assert_eq!(
        en.fork().empty_not_at_point(),
        "The agent did not fork at the chosen point. Nothing here can go on; remove it and fork again."
    );
    assert_eq!(
        ko.fork().empty_not_at_point(),
        "에이전트가 고른 지점에서 분기하지 않았습니다. 여기서는 이어 갈 수 없으니 삭제하고 다시 분기하세요."
    );
    assert_eq!(
        en.fork().empty_cannot_fork(),
        "This agent cannot fork a conversation. Nothing here can go on."
    );
    assert_eq!(
        ko.fork().empty_cannot_fork(),
        "이 에이전트는 대화를 분기할 수 없습니다. 여기서는 이어 갈 수 없습니다."
    );
    for (outcome, english, korean) in [
        (
            Notice::Refused(" ".to_owned()),
            "Failed to fork chat",
            "채팅을 분기하지 못했습니다",
        ),
        (
            Notice::Refused("refused".to_owned()),
            "Failed to fork chat: \u{2068}refused\u{2069}",
            "채팅을 분기하지 못했습니다: \u{2068}refused\u{2069}",
        ),
        (
            Notice::Origin {
                removed: true,
                message: "missing".to_owned(),
            },
            "The conversation it was forked from was removed.",
            "분기한 원래 대화는 삭제되었습니다.",
        ),
        (
            Notice::Origin {
                removed: false,
                message: " ".to_owned(),
            },
            "The conversation it was forked from could not be read. Open the session again to try again.",
            "분기한 원래 대화를 읽지 못했습니다. 세션을 다시 열면 다시 읽습니다.",
        ),
        (
            Notice::Origin {
                removed: false,
                message: " connection lost. ".to_owned(),
            },
            "The conversation it was forked from could not be read: \u{2068}connection lost\u{2069}. Open the session again to try again.",
            "분기한 원래 대화를 읽지 못했습니다: \u{2068}connection lost\u{2069}. 세션을 다시 열면 다시 읽습니다.",
        ),
    ] {
        let mut notice = PreparedNotice::new(outcome, en).unwrap();
        assert_eq!(&*notice.words(), english);
        notice.translate(ko).unwrap();
        assert_eq!(&*notice.words(), korean);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn prepared_fork_frames_and_unchanged_title_refreshes_allocate_nothing() {
    const FRAME_PROBES: usize = 10_000;
    let en = catalog("en");
    let fleet = fleet();
    let source = fleet.machines[0].sessions[0].session;
    let mut headers = Headers::default();
    assert!(headers.refresh(&fleet, en).unwrap());
    let notice = PreparedNotice::new(Notice::Refused("failure".to_owned()), en).unwrap();
    let allocations = allocation_counter::measure(|| {
        for _ in 0..FRAME_PROBES {
            let _caption = std::hint::black_box(headers.words(source, en));
            let _notice = std::hint::black_box(notice.words());
            let _plain = std::hint::black_box(en.fork().empty_not_at_point());
            assert!(!headers.refresh(&fleet, en).unwrap());
        }
    });
    assert_eq!(allocations.count_total, 0);
    assert_eq!(allocations.bytes_total, 0);
}
