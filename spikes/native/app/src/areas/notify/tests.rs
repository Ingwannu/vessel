//! The notifier on GPUI's test platform (ADR 0114): what is shown and
//! with which words, tags that carry no session, a click that opens the
//! session, the session in front left alone, and the title's count.

use gpui::{Entity, SystemNotificationResponse, TestAppContext, VisualTestContext};
use vessel_app_view::prefs::Alerts;
use vessel_app_view::{Card, Fleet, Machine, Reach};
use vessel_proto::{
    Activity, DisplayStatus, HarnessKind, KanbanColumn, SessionBoard, SessionId, SessionInfo,
    SessionMode, SessionSpec, SessionStatus,
};

use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::{Shell, language_tests};

/// A session on "lab": running, ended, failed, or waiting on a permission.
fn card(id: SessionId, status: SessionStatus, blocked: bool) -> Card {
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
    info.status = status;
    if blocked {
        info.board = Some(SessionBoard::new(
            KanbanColumn::Building,
            DisplayStatus::Blocked,
            Activity::Active,
            at,
        ));
    }
    Card::from_session("lab", &info)
}

fn fleet(cards: Vec<Card>) -> Fleet {
    Fleet {
        machines: vec![Machine {
            connection_epoch: None,
            node: None,
            name: "lab".to_owned(),
            reach: Reach::Online,
            sessions: cards,
            agents: Vec::new(),
            protocol: Some(vessel_proto::PROTOCOL_VERSION),
            version: None,
        }],
        read_at: None,
    }
}

fn running(id: SessionId) -> Card {
    card(id, SessionStatus::Running, false)
}

fn ended(id: SessionId) -> Card {
    card(id, SessionStatus::Exited { code: Some(0) }, false)
}

/// The shell with an app identity (the test platform shows nothing
/// without one, as Windows does not) and Settings' `alerts`.
fn opened(cx: &mut TestAppContext, alerts: Alerts) -> (Entity<Shell>, &mut VisualTestContext) {
    language_tests::init(cx);
    cx.update(|cx| cx.set_app_identity("Vessel.App.Test", "Vessel"));
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, _| {
        shell.prefs.settings.notifications.alerts = alerts;
    });
    (shell, cx)
}

/// The machines change from `before` to `after`.
fn change(shell: &Entity<Shell>, cx: &mut VisualTestContext, before: &Fleet, after: Fleet) {
    shell.update(cx, |shell, cx| {
        shell.fleet = after;
        shell.notify_fleet(before, cx);
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_finished_session_is_shown_and_its_click_opens_it(cx: &mut TestAppContext) {
    let (shell, cx) = opened(cx, Alerts::System);
    cx.deactivate_window();
    let id = SessionId::random();
    let after = ended(id);
    let name = if after.title.is_empty() {
        "Untitled session".to_owned()
    } else {
        after.title.clone()
    };
    change(&shell, cx, &fleet(vec![running(id)]), fleet(vec![after]));
    let shown = cx.shown_system_notifications();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].title, "Session finished");
    assert_eq!(shown[0].body, name);
    assert!(
        !shown[0].tag.contains(&id.to_string()) && !shown[0].tag.contains("lab"),
        "the tag names no session and no machine: {}",
        shown[0].tag
    );
    cx.simulate_system_notification_response(SystemNotificationResponse {
        tag: shown[0].tag.clone(),
        action_id: None,
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Session);
        assert_eq!(
            shell.areas.session.open.as_ref().map(|open| open.session),
            Some(id)
        );
    });
}

/// A tag from an earlier run, or one past the kept 64, opens nothing.
#[gpui::test]
fn an_unknown_tag_opens_nothing(cx: &mut TestAppContext) {
    let (shell, cx) = opened(cx, Alerts::System);
    cx.simulate_system_notification_response(SystemNotificationResponse {
        tag: "vessel-an-earlier-run-1".into(),
        action_id: None,
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.session.open.is_none());
    });
}

/// Paseo's rule: in front on that session, nothing; another session, or
/// the same one while away, is shown. A failure is never shown. The same
/// session and kind keeps its tag, so the system replaces it.
#[gpui::test]
fn the_session_in_front_is_left_alone(cx: &mut TestAppContext) {
    let (shell, cx) = opened(cx, Alerts::System);
    let here = SessionId::random();
    let there = SessionId::random();
    let failed = SessionId::random();
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    shell.update(cx, |shell, _| {
        shell.areas.session.open = Some(Open::of(&running(here)));
        shell.route = Route::Session;
        assert!(shell.areas.notify.active, "the window is in front");
    });
    let before = fleet(vec![running(here), running(there), running(failed)]);
    let after = fleet(vec![
        card(here, SessionStatus::Running, true),
        card(there, SessionStatus::Running, true),
        card(
            failed,
            SessionStatus::Failed {
                message: "the agent did not start".to_owned(),
                empty_fork: false,
            },
            false,
        ),
    ]);
    change(&shell, cx, &before, after.clone());
    let shown = cx.shown_system_notifications();
    assert_eq!(shown.len(), 1, "only the other session: {shown:?}");
    assert_eq!(shown[0].title, "Session needs you");
    cx.deactivate_window();
    change(&shell, cx, &before, after.clone());
    let shown = cx.shown_system_notifications();
    assert_eq!(shown.len(), 3, "away, the shown session too: {shown:?}");
    change(&shell, cx, &before, after);
    let delivered = cx.delivered_system_notifications();
    assert_eq!(delivered.len(), 2, "the same tags replaced: {delivered:?}");
}

/// The page's tab title: a count while away, whatever the alerts, gone
/// when the window comes back; Off shows nothing.
#[gpui::test]
fn the_title_counts_while_away(cx: &mut TestAppContext) {
    let (shell, cx) = opened(cx, Alerts::Off);
    cx.deactivate_window();
    let one = SessionId::random();
    let two = SessionId::random();
    change(
        &shell,
        cx,
        &fleet(vec![running(one), running(two)]),
        fleet(vec![ended(one), ended(two)]),
    );
    assert!(cx.shown_system_notifications().is_empty(), "alerts are off");
    let base = crate::window_title();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(cx.window_title(), Some(format!("(2) {base}")));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(cx.window_title(), Some(base));
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.areas.notify.unseen.count(), 0);
    });
}
