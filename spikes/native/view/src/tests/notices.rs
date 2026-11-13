//! `web/src/data/notices.test.ts`: what is worth telling, between two
//! reads of the machines.

use vessel_proto::{
    Activity, DisplayStatus, KanbanColumn, SessionBoard, SessionInfo, SessionStatus,
};

use super::fixture::{at, fleet, in_folder, machine, session as made};
use crate::Lang;
use crate::notices::{NoticeKind, notices, title_with};

/// The page's `session(id, status, live)`: running (or exited 0) in
/// `/work/{id}`, with the node's phrase when `status` is one.
fn session(short: &str, status: Option<DisplayStatus>, live: bool) -> SessionInfo {
    let process = if live {
        SessionStatus::Running
    } else {
        SessionStatus::Exited { code: Some(0) }
    };
    let mut info = in_folder(
        made(short, "2026-11-11T06:00:00Z", process),
        &format!("/work/{short}"),
    );
    info.board = status.map(|display_status| {
        SessionBoard::new(
            KanbanColumn::Building,
            display_status,
            Activity::Active,
            at("2026-11-11T06:00:00Z"),
        )
    });
    info
}

fn kinds(before: &[SessionInfo], after: &[SessionInfo]) -> Vec<NoticeKind> {
    notices(
        &fleet(vec![machine("lab", true, before)]),
        &fleet(vec![machine("lab", true, after)]),
    )
    .into_iter()
    .map(|notice| notice.kind)
    .collect()
}

#[test]
fn a_session_starts_to_need_you() {
    assert_eq!(
        kinds(
            &[session("a", Some(DisplayStatus::Working), true)],
            &[session("a", Some(DisplayStatus::Blocked), true)],
        ),
        [NoticeKind::NeedsYou]
    );
}

#[test]
fn a_working_session_goes_idle_or_exits_finished() {
    let working = [session("a", Some(DisplayStatus::Working), true)];
    assert_eq!(
        kinds(&working, &[session("a", Some(DisplayStatus::Idle), true)]),
        [NoticeKind::Finished]
    );
    assert_eq!(
        kinds(
            &working,
            &[session("a", Some(DisplayStatus::Exited), false)]
        ),
        [NoticeKind::Finished]
    );
}

#[test]
fn nothing_new_still_waiting_still_working_idle_again_a_first_sighting() {
    let blocked = [session("a", Some(DisplayStatus::Blocked), true)];
    let working = [session("a", Some(DisplayStatus::Working), true)];
    let idle = [session("a", Some(DisplayStatus::Idle), true)];
    assert_eq!(kinds(&blocked, &blocked), []);
    assert_eq!(kinds(&working, &working), []);
    assert_eq!(kinds(&idle, &idle), []);
    assert_eq!(
        kinds(&[], &[session("b", Some(DisplayStatus::Blocked), true)]),
        []
    );
}

#[test]
fn an_answered_permission_that_goes_back_to_work_says_nothing() {
    assert_eq!(
        kinds(
            &[session("a", Some(DisplayStatus::Blocked), true)],
            &[session("a", Some(DisplayStatus::Working), true)],
        ),
        []
    );
}

#[test]
fn an_older_nodes_session_finishes_when_it_ends() {
    assert_eq!(
        kinds(&[session("a", None, true)], &[session("a", None, false)]),
        [NoticeKind::Finished]
    );
}

/// The native app reads each machine on its own: one that was not read
/// before (just added, or back online) is a first sighting too.
#[test]
fn a_machine_seen_for_the_first_time_says_nothing() {
    let before = fleet(vec![machine("lab", false, &[])]);
    let after = fleet(vec![machine(
        "lab",
        true,
        &[session("a", Some(DisplayStatus::Blocked), true)],
    )]);
    assert_eq!(notices(&before, &after), []);
}

#[test]
fn titles() {
    let found = notices(
        &fleet(vec![machine(
            "lab",
            true,
            &[session("a", Some(DisplayStatus::Working), true)],
        )]),
        &fleet(vec![machine(
            "lab",
            true,
            &[session("a", Some(DisplayStatus::Blocked), true)],
        )]),
    );
    assert_eq!(found[0].title, "a");
    assert_eq!(found[0].machine, "lab");
    assert_eq!(
        Lang::En.notices().title(NoticeKind::NeedsYou),
        "Session needs you"
    );
    assert_eq!(title_with("Vessel", 2), "(2) Vessel");
    assert_eq!(title_with("Vessel", 0), "Vessel");
}
