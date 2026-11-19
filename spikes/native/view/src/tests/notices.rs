//! `web/src/data/notices.test.ts`: what is worth telling, between two
//! reads of the machines.

use vessel_proto::{
    Activity, DisplayStatus, KanbanColumn, SessionBoard, SessionId, SessionInfo, SessionStatus,
};

use super::fixture::{at, fleet, in_folder, machine, session as made};
use crate::Lang;
use crate::notices::{
    Announce, Notice, NoticeKind, Seen, Unseen, announce, notices, samples, title_with,
};
use crate::prefs::Alerts;

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

/// Paseo never notifies an error (ADR 0114): a session whose process
/// failed after working says nothing, though it rests now.
#[test]
fn a_failure_is_not_announced() {
    let mut failed = in_folder(
        made(
            "a",
            "2026-11-11T06:00:00Z",
            SessionStatus::Failed {
                message: "the agent did not start".to_owned(),
                empty_fork: false,
            },
        ),
        "/work/a",
    );
    failed.board = session("a", Some(DisplayStatus::Idle), false).board;
    assert_eq!(
        kinds(
            &[session("a", Some(DisplayStatus::Working), true)],
            &[failed]
        ),
        []
    );
}

fn notice(machine: &str, session: SessionId, kind: NoticeKind) -> Notice {
    Notice {
        kind,
        machine: machine.to_owned(),
        session,
        title: "a".to_owned(),
    }
}

fn told(found: &[Notice], seen: Seen<'_>, alerts: Alerts) -> Vec<(NoticeKind, bool, bool)> {
    announce(found.to_vec(), seen, alerts)
        .into_iter()
        .map(
            |Announce {
                 notice,
                 system,
                 sound,
             }| (notice.kind, system, sound),
        )
        .collect()
}

/// Paseo's `notifyAgentAttention`: dropped only when the app is in front
/// on that very session; Settings' choice says how the rest is told.
#[test]
fn a_notice_is_told_unless_the_person_is_on_its_session() {
    let here = SessionId::random();
    let there = SessionId::random();
    let found = [
        notice("lab", here, NoticeKind::NeedsYou),
        notice("lab", there, NoticeKind::Finished),
        notice("desk", here, NoticeKind::Finished),
    ];
    let front_here = Seen {
        active: true,
        shown: Some(("lab", here)),
    };
    let away_here = Seen {
        active: false,
        ..front_here
    };
    let front_nowhere = Seen {
        active: true,
        shown: None,
    };
    let all = Alerts::SystemAndSound;
    assert_eq!(
        told(&found, front_here, all),
        [
            (NoticeKind::Finished, true, true),
            (NoticeKind::Finished, true, true)
        ],
        "in front on lab/here: that one is dropped, the same id elsewhere is not"
    );
    assert_eq!(told(&found, away_here, all).len(), 3, "not in front");
    assert_eq!(
        told(&found, front_nowhere, all).len(),
        3,
        "in front elsewhere"
    );
    let one = &found[..1];
    assert_eq!(
        told(one, away_here, Alerts::System),
        [(NoticeKind::NeedsYou, true, false)]
    );
    assert_eq!(
        told(one, away_here, Alerts::Sound),
        [(NoticeKind::NeedsYou, false, true)]
    );
    assert_eq!(told(one, away_here, Alerts::Off), []);
}

/// The page's tab title: notices since the app was last in front, counted
/// whatever the alerts, cleared when it comes back.
#[test]
fn the_title_counts_what_came_while_away() {
    let mut unseen = Unseen::default();
    assert!(!unseen.found(2, true, true), "in front: nothing to count");
    assert!(!unseen.found(2, false, false), "the title does not count");
    assert!(!unseen.found(0, false, true));
    assert!(unseen.found(2, false, true));
    assert!(unseen.found(1, false, true));
    assert_eq!(unseen.count(), 3);
    assert!(unseen.back());
    assert_eq!(unseen.count(), 0);
    assert!(!unseen.back(), "nothing left to clear");
}

/// Zero crossings in `range` of `sound` at `rate`: about twice the pitch
/// times the length.
fn crossings(sound: &[f64], range: std::ops::Range<usize>) -> usize {
    sound[range]
        .windows(2)
        .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
        .count()
}

/// The page's two tones (`sound.ts`): 0.41 s long, quiet at its start,
/// never louder than one tone's 0.18 (the first has faded when the
/// second sounds), and needs-you rising where finished falls.
#[test]
fn a_notice_sounds_as_the_page() {
    let rate = 48_000;
    for kind in [NoticeKind::NeedsYou, NoticeKind::Finished] {
        let sound = samples(kind, rate);
        assert_eq!(sound.len(), 19_680, "{kind:?}");
        assert!(sound[0].abs() < 0.001, "{kind:?} starts quiet");
        let peak = sound
            .iter()
            .fold(0.0_f64, |peak, sample| peak.max(sample.abs()));
        assert!((0.17..=0.181).contains(&peak), "{kind:?} peak {peak}");
        // 0.04-0.12 s is the first tone alone, 0.20-0.28 s the second.
        let first = crossings(&sound, 1_920..5_760);
        let second = crossings(&sound, 9_600..13_440);
        match kind {
            NoticeKind::NeedsYou => assert!(first < second, "rises: {first} < {second}"),
            NoticeKind::Finished => assert!(first > second, "falls: {first} > {second}"),
        }
        // 660 Hz over 0.08 s crosses zero about 105 times, 880 Hz about 141.
        let low = first.min(second);
        let high = first.max(second);
        assert!((100..=110).contains(&low), "{kind:?} low {low}");
        assert!((136..=146).contains(&high), "{kind:?} high {high}");
    }
    assert_eq!(samples(NoticeKind::NeedsYou, 0), Vec::<f64>::new());
}
