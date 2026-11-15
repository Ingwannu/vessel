//! The drafts' file: owner-only, versioned, and a file that does not read
//! is set aside rather than lost.

use std::path::PathBuf;

use chrono::Utc;
use vessel_app_view::composer::drafts::{DraftKey, Drafts};
use vessel_proto::SessionId;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vessel-app-drafts-{}-{name}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    dir.join("state").join(FILE)
}

fn some_drafts() -> (Drafts, SessionId) {
    let session = SessionId::random();
    let mut drafts = Drafts::default();
    let _new = drafts.keep(DraftKey::New, "a new chat, half typed", Utc::now());
    let _session = drafts.keep(
        DraftKey::Session { session },
        "그리고 테스트도 돌려 줘",
        Utc::now(),
    );
    (drafts, session)
}

#[test]
fn drafts_written_read_back_per_composer() {
    let path = scratch("roundtrip");
    let (drafts, session) = some_drafts();
    save_at(&path, &drafts).unwrap();
    let read = load_at(&path);
    assert_eq!(read.text(DraftKey::New), "a new chat, half typed");
    assert_eq!(
        read.text(DraftKey::Session { session }),
        "그리고 테스트도 돌려 줘"
    );
    assert_eq!(
        read.text(DraftKey::Session {
            session: SessionId::random()
        }),
        ""
    );
}

#[cfg(unix)]
#[test]
fn the_file_is_owner_only_in_an_owner_only_folder() {
    use std::os::unix::fs::PermissionsExt as _;

    let path = scratch("modes");
    let (drafts, _) = some_drafts();
    save_at(&path, &drafts).unwrap();
    let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(path.parent().unwrap()), 0o700);
    save_at(&path, &drafts).unwrap();
    assert_eq!(mode(&path), 0o600, "rewritten");
    assert!(!path.with_extension("json.tmp").exists());
}

#[test]
fn a_file_that_does_not_read_is_set_aside_not_lost() {
    let path = scratch("unread");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"{ \"version\": 99, \"drafts\": {} }").unwrap();
    assert!(load_at(&path).is_empty());
    let aside = path.with_extension("json.unread");
    assert!(aside.exists(), "kept for the person");
    assert!(!path.exists());
    // The next write does not touch what was set aside.
    let (drafts, _) = some_drafts();
    save_at(&path, &drafts).unwrap();
    assert_eq!(
        std::fs::read(&aside).unwrap(),
        b"{ \"version\": 99, \"drafts\": {} }"
    );
}

#[test]
fn a_missing_file_is_no_drafts() {
    assert!(load_at(&scratch("missing")).is_empty());
}
