//! Every composer's tray: kept per session, at most sixteen, the new
//! chat's carried over to its session.

use std::path::PathBuf;

use vessel_proto::SessionId;

use super::*;
use crate::composer::tray::Origin;

fn with_file(trays: &mut Trays, key: DraftKey) {
    let _key = trays
        .tray(key)
        .add("notes.md", 10, Origin::File(PathBuf::from("/tmp/notes.md")));
}

#[test]
fn a_sessions_files_wait_for_it_while_another_is_open() {
    let mut trays = Trays::default();
    let first = DraftKey::Session {
        session: SessionId::random(),
    };
    let second = DraftKey::Session {
        session: SessionId::random(),
    };
    with_file(&mut trays, first);
    let _other = trays.tray(second);
    assert_eq!(trays.get(first).map(|tray| tray.items().len()), Some(1));
    assert!(trays.get(second).is_some_and(Tray::is_empty));
}

#[test]
fn the_least_recently_used_goes_past_sixteen() {
    let mut trays = Trays::default();
    let keys: Vec<DraftKey> = (0..=Trays::MAX)
        .map(|_| DraftKey::Session {
            session: SessionId::random(),
        })
        .collect();
    for key in &keys {
        with_file(&mut trays, *key);
    }
    assert_eq!(trays.len(), Trays::MAX);
    assert!(trays.get(keys[0]).is_none(), "the oldest went");
    assert!(trays.get(keys[Trays::MAX]).is_some(), "the newest stayed");
    // Using one makes it the newest: the next to go is another.
    let _used = trays.tray(keys[1]);
    let _new = trays.tray(DraftKey::New);
    assert!(trays.get(keys[1]).is_some());
    assert!(trays.get(keys[2]).is_none());
}

#[test]
fn the_new_chats_files_become_its_sessions() {
    let mut trays = Trays::default();
    with_file(&mut trays, DraftKey::New);
    let session = DraftKey::Session {
        session: SessionId::random(),
    };
    trays.rename(DraftKey::New, session);
    assert!(trays.get(DraftKey::New).is_none());
    assert_eq!(trays.get(session).map(|tray| tray.items().len()), Some(1));
}
