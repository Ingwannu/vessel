//! A machine's answer decides which notices are told: what it admitted and
//! what its first inbox page did not hold, never the rest. A machine that
//! admits claims is asked once for all its notices; one that does not, and
//! a window that does not notify, tell at once. At ADR 0089's protocol a
//! fleet change asks, shows nothing until the answer, never asks about the
//! session in front, and an admitted notice's click opens its session.

use gpui::{SystemNotificationResponse, TestAppContext};
use vessel_app_link::claims::{Answer, Ask};
use vessel_app_link::{Command, Link};
use vessel_app_view::notices::claims::Asked;
use vessel_app_view::notices::{Announce, Notice, NoticeKind};
use vessel_app_view::prefs::Alerts;
use vessel_proto::notifications::{ATTENTION_NOTIFICATIONS_PROTOCOL, NotificationClaim};
use vessel_proto::{
    Activity, DisplayStatus, HarnessKind, KanbanColumn, SessionBoard, SessionId, SessionInfo,
    SessionMode, SessionSpec,
};

use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::{Shell, language_tests};

fn told(title: &str) -> Announce {
    Announce {
        notice: Notice {
            kind: NoticeKind::Finished,
            machine: "lab".to_owned(),
            session: SessionId::random(),
            title: title.to_owned(),
        },
        system: true,
        sound: false,
    }
}

#[gpui::test]
fn only_what_the_machine_admitted_or_could_not_find_is_told(cx: &mut TestAppContext) {
    language_tests::init(cx);
    // The test platform shows nothing without an app identity, as Windows
    // does not (the notify tests' `opened`).
    cx.update(|cx| cx.set_app_identity("Vessel.App.Test", "Vessel"));
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.deactivate_window();
    shell.update(cx, |shell, cx| {
        shell.notices_claimed(
            Answer {
                machine: "lab".to_owned(),
                asked: vec![
                    (
                        told("admitted"),
                        Asked::Answered(NotificationClaim::Admitted),
                    ),
                    (
                        told("elsewhere"),
                        Asked::Answered(NotificationClaim::NotRecipient),
                    ),
                    (told("seen"), Asked::Answered(NotificationClaim::Suppressed)),
                    (told("past the page"), Asked::NotOnFirstPage),
                    (told("lost"), Asked::Failed),
                ],
            },
            cx,
        );
    });
    cx.run_until_parked();
    let bodies: Vec<String> = cx
        .shown_system_notifications()
        .iter()
        .map(|shown| shown.body.to_string())
        .collect();
    assert_eq!(
        bodies,
        vec!["admitted".to_owned(), "past the page".to_owned()]
    );
}

/// The lab, online at this build's protocol, listing `sessions`.
fn claiming(sessions: Vec<vessel_app_view::Card>) -> vessel_app_view::Fleet {
    vessel_app_view::Fleet {
        machines: vec![vessel_app_view::Machine {
            connection_epoch: None,
            node: None,
            name: "lab".to_owned(),
            reach: vessel_app_view::Reach::Online,
            sessions,
            agents: Vec::new(),
            protocol: Some(vessel_proto::PROTOCOL_VERSION),
            version: None,
        }],
        read_at: None,
    }
}

/// The asks among what the shell sent.
fn asks(sent: &mut tokio::sync::mpsc::Receiver<Command>) -> Vec<Ask> {
    let mut asks = Vec::new();
    while let Ok(command) = sent.try_recv() {
        if let Command::Claims(ask) = command {
            asks.push(ask);
        }
    }
    asks
}

/// A machine that admits claims hears all its notices in one ask, and none
/// is told before the answer; while the protocol is unassigned every
/// notice is told at once and nothing is asked. Both branches assert.
#[gpui::test]
fn a_claiming_machine_is_asked_once_for_all_its_notices(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut sent) = Link::detached(16).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.run_until_parked();
    let _opening = asks(&mut sent);
    let now = shell.update(cx, |shell, _cx| {
        shell.fleet = claiming(Vec::new());
        shell.prefs.settings.notifications.alerts = Alerts::System;
        shell.areas.notify.reaches_system(true);
        shell
            .ask_before_telling(vec![told("one"), told("two")])
            .len()
    });
    let asked: Vec<usize> = asks(&mut sent).iter().map(|ask| ask.told.len()).collect();
    if ATTENTION_NOTIFICATIONS_PROTOCOL.is_some() {
        assert_eq!(
            (now, asked),
            (0, vec![2]),
            "one ask for both, none told yet"
        );
    } else {
        assert_eq!((now, asked), (2, Vec::new()), "told at once, nothing asked");
    }
}

/// A session of the lab's, waiting on a permission when `blocked`.
fn session(id: SessionId, blocked: bool) -> vessel_app_view::Card {
    let at = chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap();
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let mut info = SessionInfo::new(id, spec, at);
    if blocked {
        info.board = Some(SessionBoard::new(
            KanbanColumn::Building,
            DisplayStatus::Blocked,
            Activity::Active,
            at,
        ));
    }
    vessel_app_view::Card::from_session("lab", &info)
}

/// `notify_fleet` on a machine at ADR 0089's protocol (the protocol-47
/// adapters' review): the change sends one ask and shows nothing; the
/// session in front is not asked about; the admitted notice shows; its
/// click opens its session.
#[gpui::test]
fn at_the_claims_protocol_a_fleet_change_asks_and_shows_only_what_is_admitted(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    cx.update(|cx| cx.set_app_identity("Vessel.App.Test", "Vessel"));
    let (link, mut sent) = Link::detached(16).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let _opening = asks(&mut sent);
    let (here, there) = (SessionId::random(), SessionId::random());
    shell.update(cx, |shell, cx| {
        shell.prefs.settings.notifications.alerts = Alerts::System;
        shell.areas.notify.reaches_system(true);
        shell.areas.session.open = Some(Open::of(&session(here, false)));
        shell.route = Route::Session;
        assert!(shell.areas.notify.active, "the window is in front");
        let before = claiming(vec![session(here, false), session(there, false)]);
        shell.fleet = claiming(vec![session(here, true), session(there, true)]);
        shell.notify_fleet(&before, cx);
    });
    cx.run_until_parked();
    assert!(
        cx.shown_system_notifications().is_empty(),
        "nothing shows before the machine answers"
    );
    let mut asked = asks(&mut sent);
    let assigned = ATTENTION_NOTIFICATIONS_PROTOCOL.is_some();
    if !assigned {
        // Below the protocol the change is told at once, as ADR 0114's
        // notifier tests cover; nothing here is asked.
        assert!(asked.is_empty());
        return;
    }
    assert_eq!(asked.len(), 1, "one ask for the change");
    let ask = asked.remove(0);
    let sessions: Vec<SessionId> = ask.told.iter().map(|one| one.notice.session).collect();
    assert_eq!(
        sessions,
        vec![there],
        "the session in front is not asked about"
    );
    shell.update(cx, |shell, cx| {
        let asked = ask
            .told
            .into_iter()
            .map(|one| (one, Asked::Answered(NotificationClaim::Admitted)))
            .collect();
        shell.notices_claimed(
            Answer {
                machine: ask.machine,
                asked,
            },
            cx,
        );
    });
    cx.run_until_parked();
    let shown = cx.shown_system_notifications();
    assert_eq!(shown.len(), 1, "the admitted notice: {shown:?}");
    assert_eq!(shown[0].title, "Session needs you");
    cx.simulate_system_notification_response(SystemNotificationResponse {
        tag: shown[0].tag.clone(),
        action_id: None,
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell.areas.session.open.as_ref().map(|open| open.session),
            Some(there)
        );
    });
}

/// A window that does not notify never claims, even on a machine that
/// admits claims: every notice takes ADR 0114's local path (ADR 0089,
/// "Turned on", item 5). A window that notifies asks a claiming machine.
#[gpui::test]
fn a_window_that_does_not_notify_tells_at_once_without_asking(cx: &mut TestAppContext) {
    use vessel_app_view::{Fleet, Machine, Reach};
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, _cx| {
        shell.fleet = Fleet {
            machines: vec![Machine {
                connection_epoch: None,
                node: None,
                name: "lab".to_owned(),
                reach: Reach::Online,
                sessions: Vec::new(),
                agents: Vec::new(),
                protocol: Some(vessel_proto::PROTOCOL_VERSION),
                version: None,
            }],
            read_at: None,
        };
        for (alerts, reaches) in [(Alerts::Sound, true), (Alerts::System, false)] {
            shell.prefs.settings.notifications.alerts = alerts;
            shell.areas.notify.reaches_system(reaches);
            assert_eq!(
                shell
                    .ask_before_telling(vec![told("one"), told("two")])
                    .len(),
                2,
                "{alerts:?}, system reached {reaches}: told at once"
            );
        }
        shell.prefs.settings.notifications.alerts = Alerts::System;
        shell.areas.notify.reaches_system(true);
        let assigned = ATTENTION_NOTIFICATIONS_PROTOCOL.is_some();
        assert_eq!(
            shell.ask_before_telling(vec![told("one")]).is_empty(),
            assigned,
            "a window that notifies asks a machine that admits claims"
        );
    });
}
