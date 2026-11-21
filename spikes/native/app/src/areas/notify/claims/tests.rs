//! A machine's answer decides which notices are told: what it admitted and
//! what its first inbox page did not hold, never the rest. While ADR
//! 0089's protocol is unassigned no machine is asked, so a notice is told
//! at once, as it is by a window that does not notify.

use gpui::TestAppContext;
use vessel_app_link::claims::Answer;
use vessel_app_view::notices::claims::Asked;
use vessel_app_view::notices::{Announce, Notice, NoticeKind};
use vessel_proto::SessionId;
use vessel_proto::notifications::NotificationClaim;

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

#[gpui::test]
fn while_unassigned_every_notice_is_told_at_once(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, _cx| {
        let now = shell.ask_before_telling(vec![told("one"), told("two")]);
        if vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
            assert_eq!(now.len(), 2);
        }
    });
}

/// A window that does not notify never claims, even on a machine that
/// admits claims: every notice takes ADR 0114's local path (ADR 0089,
/// "Turned on", item 5). A window that notifies asks a claiming machine.
#[gpui::test]
fn a_window_that_does_not_notify_tells_at_once_without_asking(cx: &mut TestAppContext) {
    use vessel_app_view::prefs::Alerts;
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
        let assigned = vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_some();
        assert_eq!(
            shell.ask_before_telling(vec![told("one")]).is_empty(),
            assigned,
            "a window that notifies asks a machine that admits claims"
        );
    });
}
