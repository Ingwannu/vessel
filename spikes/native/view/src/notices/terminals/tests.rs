//! A connection's first terminal list tells nothing; a later one tells each
//! Finished or Needs-input attention at a revision not told before, never
//! a failed terminal's, and never a command.

use std::path::PathBuf;

use chrono::Utc;
use vessel_proto::notifications::{
    NotificationSource, TerminalActivity, TerminalAttention, TerminalAttentionReason,
    TerminalAttentionRevision,
};
use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalPlace};
use vessel_proto::{SessionId, SessionStatus, SessionTitle, TerminalId};

use super::{TerminalNotice, Watched, announce_terminals, to_clear};
use crate::notices::NoticeKind;
use crate::prefs::Alerts;

fn terminal(id: TerminalId, title: Option<&str>) -> TerminalInfo {
    let info = TerminalInfo::new(
        id,
        TerminalPlace::Session {
            session: SessionId::random(),
        },
        PathBuf::from("/work"),
        TerminalKind::Shell {
            program: PathBuf::from("/bin/sh"),
        },
        Utc::now(),
    );
    info.with_title(title.map(|text| text.parse::<SessionTitle>().unwrap()))
}

fn asking(
    mut info: TerminalInfo,
    revision: u64,
    reason: Option<TerminalAttentionReason>,
) -> TerminalInfo {
    info.attention = Some(TerminalAttention {
        terminal: info.id,
        revision: TerminalAttentionRevision::new(revision),
        activity: reason.map_or(TerminalActivity::Working, |reason| {
            TerminalActivity::Attention { reason }
        }),
        changed_at: Utc::now(),
    });
    info
}

#[test]
fn the_first_list_tells_nothing_and_a_new_revision_is_told_once() {
    let id = TerminalId::random();
    let mut watched = Watched::default();
    let waiting = asking(
        terminal(id, Some("build")),
        3,
        Some(TerminalAttentionReason::Finished),
    );
    assert!(
        watched
            .read("lab", std::slice::from_ref(&waiting))
            .is_empty()
    );
    assert!(
        watched
            .read("lab", std::slice::from_ref(&waiting))
            .is_empty(),
        "the same revision again"
    );
    let asked = asking(
        terminal(id, Some("build")),
        4,
        Some(TerminalAttentionReason::NeedsInput),
    );
    let told = watched.read("lab", std::slice::from_ref(&asked));
    assert_eq!(told.len(), 1);
    let notice: &TerminalNotice = &told[0];
    assert_eq!(notice.kind, NoticeKind::NeedsYou);
    assert_eq!(notice.machine, "lab");
    assert_eq!(notice.title, "build");
    assert_eq!(
        notice.source(),
        NotificationSource::Terminal {
            terminal: id,
            revision: TerminalAttentionRevision::new(4),
        }
    );
    assert!(watched.read("lab", &[asked]).is_empty(), "told once");
}

#[test]
fn only_finished_or_needs_input_of_a_terminal_that_did_not_fail_is_told() {
    let mut watched = Watched::default();
    let (working, unknown, failed, finished, untitled) = (
        TerminalId::random(),
        TerminalId::random(),
        TerminalId::random(),
        TerminalId::random(),
        TerminalId::random(),
    );
    assert!(watched.read("lab", &[]).is_empty());
    let mut broke = asking(
        terminal(failed, Some("broke")),
        2,
        Some(TerminalAttentionReason::Finished),
    );
    broke.status = SessionStatus::failed("gone");
    let list = [
        asking(terminal(working, Some("busy")), 2, None),
        asking(
            terminal(unknown, Some("odd")),
            2,
            Some(TerminalAttentionReason::Unknown),
        ),
        broke,
        asking(
            terminal(finished, Some("done")),
            2,
            Some(TerminalAttentionReason::Finished),
        ),
        asking(
            terminal(untitled, None),
            5,
            Some(TerminalAttentionReason::NeedsInput),
        ),
    ];
    let told: Vec<(TerminalId, NoticeKind, String)> = watched
        .read("lab", &list)
        .into_iter()
        .map(|notice| (notice.terminal, notice.kind, notice.title))
        .collect();
    assert_eq!(
        told,
        vec![
            (finished, NoticeKind::Finished, "done".to_owned()),
            (untitled, NoticeKind::NeedsYou, String::new()),
        ]
    );
}

#[test]
fn a_window_that_tells_nothing_claims_nothing() {
    let mut watched = Watched::default();
    let id = TerminalId::random();
    assert!(watched.read("lab", &[]).is_empty());
    let found = watched.read(
        "lab",
        &[asking(
            terminal(id, Some("t")),
            1,
            Some(TerminalAttentionReason::Finished),
        )],
    );
    assert!(announce_terminals(found.clone(), Alerts::Off).is_empty());
    let sound = announce_terminals(found, Alerts::Sound);
    assert_eq!(sound.len(), 1);
    assert!(!sound[0].system);
    assert!(sound[0].sound);
}

#[test]
fn viewing_clears_only_attention_at_the_exact_revision_shown() {
    let id = TerminalId::random();
    let asked = asking(
        terminal(id, None),
        7,
        Some(TerminalAttentionReason::NeedsInput),
    );
    let clear = to_clear(&asked).unwrap();
    assert_eq!(clear.terminal, id);
    assert_eq!(clear.revision, TerminalAttentionRevision::new(7));
    assert_eq!(to_clear(&asking(terminal(id, None), 8, None)), None);
    assert_eq!(to_clear(&terminal(id, None)), None);
}
