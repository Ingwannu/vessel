//! `web/src/data/sidebar.test.ts`: what the sidebar lists, in the view
//! the person chose.

use vessel_proto::{SessionInfo, SessionStatus};

use super::fixture::{at, fleet, id, in_folder, machine, session, short};
use crate::prefs::{Grouping, SessionSort, SidebarView};
use crate::sidebar::{SIDEBAR_SESSIONS, SIDEBAR_STATUS_SESSIONS, SessionMark, Step, step_session};
use crate::{Card, Fleet, Lang};

fn running(short: &str, when: &str) -> SessionInfo {
    in_folder(
        session(short, when, SessionStatus::Running),
        &format!("/work/{short}"),
    )
}

fn book(laptop: &[SessionInfo]) -> Fleet {
    fleet(vec![
        machine("laptop", true, laptop),
        machine("gpu-box", false, &[]),
    ])
}

fn state() -> Fleet {
    let mut aaaa = running("aaaa", "2026-11-11T08:00:00Z");
    aaaa.title = Some("Fix the overview".parse().unwrap());
    let mut cccc = running("cccc", "2026-11-11T07:00:00Z");
    cccc.status = SessionStatus::Exited { code: Some(0) };
    book(&[aaaa, running("bbbb", "2026-11-11T09:00:00Z"), cccc])
}

/// The page's default view, which hides ended sessions: these tests are
/// the page's (`sidebar.test.ts`), written against it. The native default
/// lists them (`SidebarView::default`, 2026-11-14); that default is pinned
/// in `tests/prefs.rs`.
fn hiding_ended() -> SidebarView {
    SidebarView {
        ended: false,
        ..SidebarView::default()
    }
}

fn ids(cards: &[&Card]) -> Vec<String> {
    cards.iter().map(|card| short(card, 4)).collect()
}

#[test]
fn lists_each_machine_with_its_live_sessions_newest_first_by_title() {
    let fleet = state();
    let machines = fleet.sidebar_machines(hiding_ended(), SIDEBAR_SESSIONS);
    let [laptop, gpu] = machines.as_slice() else {
        panic!("two machines");
    };
    assert_eq!(laptop.machine.name, "laptop");
    assert!(laptop.connected);
    assert_eq!((laptop.live, laptop.more), (2, 0));
    let titles: Vec<&str> = laptop
        .sessions
        .iter()
        .map(|card| card.title.as_str())
        .collect();
    assert_eq!(titles, ["bbbb", "Fix the overview"]);
    assert!(!gpu.connected);
    assert_eq!(gpu.live, 0);
    assert!(gpu.sessions.is_empty());
}

#[test]
fn links_to_the_board_for_the_rest() {
    let fleet = state();
    let machines = fleet.sidebar_machines(hiding_ended(), 1);
    assert_eq!(machines[0].sessions.len(), 1);
    assert_eq!(machines[0].more, 1);
}

#[test]
fn lists_ended_sessions_when_asked_and_sorts_by_activity() {
    let fleet = state();
    let view = SidebarView {
        group: Grouping::Machine,
        sort: SessionSort::Activity,
        ended: true,
    };
    let machines = fleet.sidebar_machines(view, SIDEBAR_SESSIONS);
    let seen: Vec<(String, SessionMark)> = machines[0]
        .sessions
        .iter()
        .map(|card| (short(card, 4), card.session_mark()))
        .collect();
    assert_eq!(
        seen,
        [
            ("bbbb".to_owned(), SessionMark::Idle),
            ("aaaa".to_owned(), SessionMark::Idle),
            ("cccc".to_owned(), SessionMark::Ended),
        ]
    );
}

#[test]
fn groups_by_status_across_machines_leaving_empty_groups_out() {
    let fleet = state();
    let view = SidebarView {
        group: Grouping::Status,
        ..hiding_ended()
    };
    let groups = fleet.sidebar_statuses(view, SIDEBAR_STATUS_SESSIONS);
    let labels: Vec<&str> = groups
        .iter()
        .map(|group| Lang::En.sidebar().mark(group.mark))
        .collect();
    assert_eq!(labels, ["Idle"]);
    let machines: Vec<&str> = groups[0]
        .sessions
        .iter()
        .map(|card| card.machine.as_str())
        .collect();
    assert_eq!(machines, ["laptop", "laptop"]);
}

#[test]
fn walks_the_sessions_in_the_order_shown_wrapping() {
    let fleet = state();
    let order = fleet.sidebar_order(hiding_ended());
    assert_eq!(ids(&order), ["bbbb", "aaaa"]);
    let step = |current: Option<&str>, step| {
        step_session(&order, current.map(|short| ("laptop", id(short))), step)
            .map(|card| short(card, 4))
    };
    assert_eq!(step(Some("aaaa"), Step::Next).as_deref(), Some("bbbb"));
    assert_eq!(step(Some("bbbb"), Step::Previous).as_deref(), Some("aaaa"));
    assert_eq!(step(None, Step::Previous).as_deref(), Some("aaaa"));
    assert!(step_session(&[], None, Step::Next).is_none());
}

#[test]
fn is_idle_for_a_session_of_a_node_that_does_not_place_it() {
    let card = Card::from_session("laptop", &running("aaaa", "2026-11-11T08:00:00Z"));
    assert_eq!(card.session_mark(), SessionMark::Idle);
}

#[test]
fn leaves_archived_sessions_out_lists_pinned_first_says_unread() {
    let mut bbbb = running("bbbb", "2026-11-11T09:00:00Z");
    bbbb.marks.archived_at = Some(at("2026-11-11T10:00:00Z"));
    let mut dddd = running("dddd", "2026-11-11T06:00:00Z");
    dddd.marks.pinned = true;
    dddd.marks.unread = true;
    let fleet = book(&[running("aaaa", "2026-11-11T08:00:00Z"), bbbb, dddd]);
    let machines = fleet.sidebar_machines(hiding_ended(), SIDEBAR_SESSIONS);
    let rows = &machines[0].sessions;
    assert_eq!(ids(rows), ["dddd", "aaaa"]);
    assert!(rows[0].detail.marks.pinned && rows[0].detail.marks.unread);
    assert!(!rows[1].detail.marks.pinned && !rows[1].detail.marks.unread);
}
