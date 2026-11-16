use std::time::{Duration, Instant};

use vessel_proto::workspace::panel_files::{FileRevision, FileSize, ObservedFile};
use vessel_proto::workspace::{ContentHash, RelPath, WorkspaceRoot};
use vessel_proto::{ErrorCode, ProjectId};

use super::{Answered, Watch};
use crate::workspace::pace::QUIET;
use crate::workspace::problem::Problem;

fn hash(byte: u8) -> ContentHash {
    ContentHash::from_sha256([byte; 32])
}

fn ready(byte: u8) -> ObservedFile {
    ObservedFile::Ready {
        revision: FileRevision {
            size: FileSize(u64::from(byte)),
            hash: hash(byte),
        },
    }
}

fn root() -> WorkspaceRoot {
    WorkspaceRoot::Project {
        project: ProjectId::random(),
    }
}

fn path() -> RelPath {
    "src/main.rs".parse().unwrap()
}

/// A watch of a file whose first read landed with content `seen`.
fn read(seen: Option<ContentHash>) -> Watch {
    let mut watch = Watch::default();
    watch.seen(seen);
    watch
}

/// A watch that has asked once and been answered `first` (shown: `shown`).
fn anchored(root: WorkspaceRoot, first: ObservedFile, shown: Option<&ContentHash>) -> Watch {
    let mut watch = read(None);
    let now = Instant::now();
    let ask = watch.ask(root, path(), now).unwrap();
    let Answered::Quiet { rest, .. } = watch.answered(&ask, Ok(first), shown, now) else {
        panic!("the first answer matched what is shown");
    };
    watch.rested(rest);
    watch
}

#[test]
fn nothing_is_asked_before_a_read_of_the_file_landed() {
    let root = root();
    let now = Instant::now();
    let mut watch = Watch::default();
    // An answer now could anchor a newer disk than the read in flight.
    assert_eq!(watch.ask(root, path(), now), None);
    watch.seen(Some(hash(1)));
    assert!(watch.ask(root, path(), now).is_some());
}

#[test]
fn one_observation_at_a_time_sending_back_the_last_answer() {
    let root = root();
    let mut watch = read(None);
    let now = Instant::now();
    let first = watch.ask(root, path(), now).unwrap();
    assert_eq!(first.after, None);
    assert_eq!(watch.ask(root, path(), now), None, "one is out");
    let Answered::Quiet { rest, wait } = watch.answered(&first, Ok(ready(1)), None, now) else {
        panic!("nothing shown yet is no change");
    };
    assert_eq!(wait, QUIET, "an answer at once waits out the pause");
    assert_eq!(watch.ask(root, path(), now), None, "resting");
    watch.rested(rest.wrapping_add(1));
    assert_eq!(watch.ask(root, path(), now), None, "an older pause's end");
    watch.rested(rest);
    let second = watch.ask(root, path(), now).unwrap();
    assert_eq!(second.after, Some(ready(1)));
}

#[test]
fn an_answer_after_the_pause_asks_again_at_once() {
    let mut watch = read(None);
    let asked = Instant::now();
    let ask = watch.ask(root(), path(), asked).unwrap();
    let later = asked.checked_add(Duration::from_secs(30)).unwrap();
    assert!(matches!(
        watch.answered(&ask, Ok(ready(1)), None, later),
        Answered::Quiet {
            wait: Duration::ZERO,
            ..
        }
    ));
}

#[test]
fn a_change_on_disk_is_read_again_and_the_same_state_is_not() {
    let root = root();
    let shown = hash(1);
    let mut watch = anchored(root, ready(1), Some(&shown));
    let now = Instant::now();
    let ask = watch.ask(root, path(), now).unwrap();
    let Answered::Quiet { rest, .. } = watch.answered(&ask, Ok(ready(1)), Some(&shown), now) else {
        panic!("a heartbeat with the same state is quiet");
    };
    watch.rested(rest);
    let ask = watch.ask(root, path(), now).unwrap();
    assert!(matches!(
        watch.answered(&ask, Ok(ready(2)), Some(&shown), now),
        Answered::Changed { .. }
    ));
}

#[test]
fn the_editor_s_own_save_is_no_change() {
    let root = root();
    let mut watch = anchored(root, ready(1), Some(&hash(1)));
    let now = Instant::now();
    let ask = watch.ask(root, path(), now).unwrap();
    // The editor saved: it holds version 2, which the node now reports.
    assert!(matches!(
        watch.answered(&ask, Ok(ready(2)), Some(&hash(2)), now),
        Answered::Quiet { .. }
    ));
}

#[test]
fn the_first_answer_differing_from_what_is_shown_is_a_change() {
    let root = root();
    let now = Instant::now();
    let mut watch = read(Some(hash(1)));
    let ask = watch.ask(root, path(), now).unwrap();
    assert!(matches!(
        watch.answered(&ask, Ok(ready(2)), None, now),
        Answered::Changed { .. }
    ));
    let mut gone = read(None);
    let ask = gone.ask(root, path(), now).unwrap();
    assert!(matches!(
        gone.answered(&ask, Ok(ObservedFile::Missing), None, now),
        Answered::Changed { .. }
    ));
}

#[test]
fn a_deleted_file_is_a_change_once() {
    let root = root();
    let shown = hash(1);
    let mut watch = anchored(root, ready(1), Some(&shown));
    let now = Instant::now();
    let ask = watch.ask(root, path(), now).unwrap();
    let Answered::Changed { rest, .. } =
        watch.answered(&ask, Ok(ObservedFile::Missing), Some(&shown), now)
    else {
        panic!("a deletion is a change");
    };
    watch.rested(rest);
    let ask = watch.ask(root, path(), now).unwrap();
    assert_eq!(ask.after, Some(ObservedFile::Missing));
    assert!(matches!(
        watch.answered(&ask, Ok(ObservedFile::Missing), Some(&shown), now),
        Answered::Quiet { .. }
    ));
}

#[test]
fn a_stale_answer_changes_nothing() {
    let now = Instant::now();
    let mut earlier = read(None);
    let old = earlier.ask(root(), path(), now).unwrap();
    let mut watch = read(None);
    let out = watch
        .ask(old.root, "other.rs".parse().unwrap(), now)
        .unwrap();
    assert_eq!(
        watch.answered(&old, Ok(ready(1)), None, now),
        Answered::Stale
    );
    assert_eq!(
        watch.ask(old.root, path(), now),
        None,
        "its own is still out"
    );
    assert!(matches!(
        watch.answered(&out, Ok(ready(1)), None, now),
        Answered::Quiet { .. }
    ));
    assert_eq!(
        watch.answered(&out, Ok(ready(2)), None, now),
        Answered::Stale,
        "answered once"
    );
}

#[test]
fn busy_observers_are_asked_again_after_the_pause_from_the_same_anchor() {
    let root = root();
    let shown = hash(1);
    let mut watch = anchored(root, ready(1), Some(&shown));
    let now = Instant::now();
    let ask = watch.ask(root, path(), now).unwrap();
    let busy = Problem::Node {
        code: ErrorCode::Unavailable,
        message: "workspace file observers are busy".to_owned(),
    };
    let Answered::Quiet { rest, wait } = watch.answered(&ask, Err(busy), Some(&shown), now) else {
        panic!("busy observers are asked again");
    };
    assert_eq!(wait, QUIET);
    assert!(!watch.stopped());
    watch.rested(rest);
    let again = watch.ask(root, path(), now).unwrap();
    assert_eq!(again.after, Some(ready(1)));
}

#[test]
fn a_failed_observation_stops_the_watch() {
    let root = root();
    for problem in [
        Problem::Older,
        Problem::Gone("closed".to_owned()),
        Problem::Node {
            code: ErrorCode::Forbidden,
            message: "not allowed".to_owned(),
        },
    ] {
        let mut watch = read(None);
        let now = Instant::now();
        let ask = watch.ask(root, path(), now).unwrap();
        assert_eq!(
            watch.answered(&ask, Err(problem), None, now),
            Answered::Stopped
        );
        assert!(watch.stopped());
        assert_eq!(watch.ask(root, path(), now), None);
    }
}
