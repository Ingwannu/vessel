use vessel_proto::SessionId;

use super::*;

fn read(text: &str) -> DraftSet {
    match DraftSet::decode(text.as_bytes()).unwrap() {
        Decoded::Current(set) => set,
        Decoded::Newer { version, .. } => panic!("read as newer ({version})"),
    }
}

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

fn text(text: &str) -> DraftText {
    DraftText::try_from(text.to_owned()).unwrap()
}

fn save(words: &str, when: &str) -> Save {
    Save {
        text: text(words),
        at: at(when),
    }
}

fn session() -> DraftKey {
    DraftKey::Session {
        session: SessionId::random(),
    }
}

fn texts(set: &DraftSet) -> Vec<String> {
    set.state(Kept::ThisLaunch {
        why: crate::wire::LaunchOnly::NoPlace,
    })
    .drafts
    .into_iter()
    .map(|draft| draft.text.as_str().to_owned())
    .collect()
}

#[test]
fn the_later_save_wins_whichever_process_made_it() {
    let mut set = read(
        r#"{"version":1,"drafts":[{"key":{"kind":"new"},"text":"theirs","at":"2026-11-11T10:00:00Z"}]}"#,
    );
    // Ours is older: theirs stays.
    set.merge(&BTreeMap::from([(
        DraftKey::New,
        save("ours", "2026-11-11T09:00:00Z"),
    )]));
    assert_eq!(set.text(&DraftKey::New).unwrap().as_str(), "theirs");
    // Ours is newer: ours.
    set.merge(&BTreeMap::from([(
        DraftKey::New,
        save("ours", "2026-11-11T11:00:00Z"),
    )]));
    assert_eq!(set.text(&DraftKey::New).unwrap().as_str(), "ours");
    // An older removal does not take a newer draft; a newer one does.
    set.merge(&BTreeMap::from([(
        DraftKey::New,
        save("", "2026-11-11T10:30:00Z"),
    )]));
    assert!(set.text(&DraftKey::New).is_some());
    set.merge(&BTreeMap::from([(
        DraftKey::New,
        save("", "2026-11-11T12:00:00Z"),
    )]));
    assert!(set.text(&DraftKey::New).is_none());
}

#[test]
fn unknown_entries_and_keys_are_written_back() {
    let mut set = read(
        r#"{"version":1,"device":"desk",
            "drafts":[{"key":{"kind":"thread","thread":7},"text":"later","at":"2026-11-11T10:00:00Z"},
                      {"key":{"kind":"new"},"text":"mine","at":"2026-11-11T10:00:00Z"},
                      {"key":{"kind":"new"},"text":"","at":"2026-11-11T11:00:00Z"}]}"#,
    );
    // The empty entry is no draft; the later non-empty one stands.
    assert_eq!(texts(&set), ["mine"]);
    set.merge(&BTreeMap::from([(
        session(),
        save("more", "2026-11-11T12:00:00Z"),
    )]));
    set.bound(at("2026-11-12T00:00:00Z"));
    let written = String::from_utf8(set.encode().unwrap()).unwrap();
    assert!(
        written.starts_with(r#"{"version":1,"drafts":["#),
        "{written}"
    );
    for kept in [
        r#"{"key":{"kind":"thread","thread":7},"text":"later","at":"2026-11-11T10:00:00Z"}"#,
        r#""device":"desk""#,
        r#""text":"mine""#,
        r#""text":"more""#,
    ] {
        assert!(written.contains(kept), "{kept} in {written}");
    }
}

#[test]
fn a_newer_file_or_a_broken_one_is_told_apart() {
    let decoded = DraftSet::decode(
        br#"{"version":3,"drafts":[{"key":{"kind":"new"},"text":"hi","at":"2026-11-11T10:00:00Z"}]}"#,
    )
    .unwrap();
    let Decoded::Newer { version, set } = decoded else {
        panic!("{decoded:?}");
    };
    assert_eq!((version, texts(&set)), (3, vec!["hi".to_owned()]));
    for bad in [r#"{"drafts":{}}"#, "[]", r#"{"version":0}"#, "not json"] {
        assert!(DraftSet::decode(bad.as_bytes()).is_err(), "{bad}");
    }
}

#[test]
fn drafts_unsaved_for_thirty_days_go() {
    let mut set = read(
        r#"{"drafts":[{"key":{"kind":"new"},"text":"old","at":"2026-10-01T00:00:00Z"},
                      {"key":{"kind":"later"},"text":"undated"},
                      {"key":{"kind":"later"},"at":"2026-10-01T00:00:00Z"}]}"#,
    );
    set.merge(&BTreeMap::from([(
        session(),
        save("new", "2026-11-10T00:00:00Z"),
    )]));
    set.bound(at("2026-11-11T00:00:00Z"));
    assert_eq!(texts(&set), ["new"]);
    let written = String::from_utf8(set.encode().unwrap()).unwrap();
    // A kept entry whose time does not read is not aged out; one whose
    // time reads is.
    assert!(
        written.contains("undated") && !written.contains("2026-10-01"),
        "{written}"
    );
}

#[test]
fn past_the_count_the_oldest_go_first() {
    let mut set = DraftSet::default();
    let keys: Vec<DraftKey> = (0..65).map(|_| session()).collect();
    let saves: BTreeMap<DraftKey, Save> = keys
        .iter()
        .enumerate()
        .map(|(minute, key)| {
            (
                *key,
                save("text", &format!("2026-11-11T10:{:02}:00Z", minute.min(59))),
            )
        })
        .collect();
    set.merge(&saves);
    set.bound(at("2026-11-11T12:00:00Z"));
    let kept = set.state(Kept::ThisLaunch {
        why: crate::wire::LaunchOnly::NoPlace,
    });
    assert_eq!(kept.drafts.len(), MAX_DRAFTS);
    let first = keys.first().unwrap();
    assert!(kept.drafts.iter().all(|draft| draft.key != *first));
}

#[test]
fn past_the_size_the_oldest_go_first_and_the_file_reads_back() {
    let mut set = DraftSet::default();
    let big = "x".repeat(crate::wire::MAX_DRAFT);
    let saves: BTreeMap<DraftKey, Save> = (0..9)
        .map(|n| (session(), save(&big, &format!("2026-11-11T10:0{n}:00Z"))))
        .collect();
    let oldest = *saves.iter().min_by_key(|(_key, save)| save.at).unwrap().0;
    set.merge(&saves);
    set.bound(at("2026-11-11T12:00:00Z"));
    let count = set
        .state(Kept::ThisLaunch {
            why: crate::wire::LaunchOnly::NoPlace,
        })
        .drafts
        .len();
    assert_eq!(count, 7, "seven drafts at the limit fit in the file");
    assert!(set.text(&oldest).is_none());
    let bytes = set.encode().unwrap();
    assert!(u64::try_from(bytes.len()).unwrap() <= MAX_FILE);
    assert!(matches!(DraftSet::decode(&bytes), Ok(Decoded::Current(_))));
}
