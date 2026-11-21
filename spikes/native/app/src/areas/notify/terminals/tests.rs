//! A terminal notice is claimed only by a window that would tell it, and
//! shown only when the machine admits it: titled in Paseo's words, with the
//! terminal's title (never its command) as the body.

use std::path::PathBuf;

use gpui::TestAppContext;
use vessel_app_link::Command;
use vessel_app_link::Link;
use vessel_app_link::claims::TerminalAnswer;
use vessel_app_view::notices::NoticeKind;
use vessel_app_view::notices::claims::Asked;
use vessel_app_view::notices::terminals::{TerminalAnnounce, TerminalNotice};
use vessel_app_view::prefs::Alerts;
use vessel_app_view::{Fleet, Machine, Reach};
use vessel_proto::notifications::{
    ATTENTION_NOTIFICATIONS_PROTOCOL, ClearTerminalAttention, NotificationClaim, TerminalActivity,
    TerminalAttention, TerminalAttentionReason, TerminalAttentionRevision,
};
use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalPlace};
use vessel_proto::{SessionId, TerminalId};

use crate::shell::{Shell, language_tests};

fn notice(kind: NoticeKind, title: &str) -> TerminalNotice {
    TerminalNotice {
        kind,
        machine: "lab".to_owned(),
        terminal: TerminalId::random(),
        revision: TerminalAttentionRevision::new(4),
        place: TerminalPlace::Session {
            session: SessionId::random(),
        },
        title: title.to_owned(),
    }
}

fn told(kind: NoticeKind, title: &str) -> TerminalAnnounce {
    TerminalAnnounce {
        notice: notice(kind, title),
        system: true,
        sound: false,
    }
}

#[gpui::test]
fn only_an_admitted_terminal_notice_is_shown_with_its_title(cx: &mut TestAppContext) {
    language_tests::init(cx);
    // The test platform shows nothing without an app identity, as Windows
    // does not (the notify tests' `opened`).
    cx.update(|cx| cx.set_app_identity("Vessel.App.Test", "Vessel"));
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.deactivate_window();
    shell.update(cx, |shell, cx| {
        shell.terminal_notices_claimed(
            TerminalAnswer {
                machine: "lab".to_owned(),
                asked: vec![
                    (
                        told(NoticeKind::Finished, "build"),
                        Asked::Answered(NotificationClaim::Admitted),
                    ),
                    (
                        told(NoticeKind::Finished, "elsewhere"),
                        Asked::Answered(NotificationClaim::NotRecipient),
                    ),
                    (
                        told(NoticeKind::NeedsYou, "seen"),
                        Asked::Answered(NotificationClaim::Suppressed),
                    ),
                    (told(NoticeKind::Finished, "lost"), Asked::Failed),
                    (
                        told(NoticeKind::NeedsYou, ""),
                        Asked::Answered(NotificationClaim::Admitted),
                    ),
                ],
            },
            cx,
        );
    });
    cx.run_until_parked();
    let shown: Vec<(String, String)> = cx
        .shown_system_notifications()
        .iter()
        .map(|shown| (shown.title.to_string(), shown.body.to_string()))
        .collect();
    assert_eq!(
        shown,
        vec![
            ("Terminal finished".to_owned(), "build".to_owned()),
            ("Terminal needs input".to_owned(), "Terminal".to_owned()),
        ]
    );
}

#[gpui::test]
fn a_window_that_tells_nothing_never_claims(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut sent) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.run_until_parked();
    // What the shell asks as it opens is not this test's.
    while sent.try_recv().is_ok() {}
    shell.update(cx, |shell, _cx| {
        shell.prefs.settings.notifications.alerts = Alerts::Off;
        shell.terminal_notices_found("lab".to_owned(), vec![notice(NoticeKind::Finished, "t")]);
    });
    assert!(sent.try_recv().is_err(), "alerts off: nothing is claimed");
    shell.update(cx, |shell, _cx| {
        shell.prefs.settings.notifications.alerts = Alerts::System;
        shell.terminal_notices_found("lab".to_owned(), vec![notice(NoticeKind::Finished, "t")]);
    });
    let Ok(Command::TerminalClaims(ask)) = sent.try_recv() else {
        panic!("expected the terminal claims");
    };
    assert_eq!(ask.machine, "lab");
    assert_eq!(ask.told.len(), 1);
    assert!(ask.told[0].system);
}

#[gpui::test]
fn a_viewed_terminal_clears_its_attention_once_on_a_machine_that_admits_claims(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut sent) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.run_until_parked();
    while sent.try_recv().is_ok() {}
    let id = TerminalId::random();
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
        let mut info = TerminalInfo::new(
            id,
            TerminalPlace::Session {
                session: SessionId::random(),
            },
            PathBuf::from("/work"),
            TerminalKind::Shell {
                program: PathBuf::from("/bin/sh"),
            },
            chrono::Utc::now(),
        );
        info.attention = Some(TerminalAttention {
            terminal: id,
            revision: TerminalAttentionRevision::new(7),
            activity: TerminalActivity::Attention {
                reason: TerminalAttentionReason::NeedsInput,
            },
            changed_at: chrono::Utc::now(),
        });
        shell.areas.terminal_tabs.listed_for_test(info);
        shell.terminal_attention_seen("lab", id);
        shell.terminal_attention_seen("lab", id);
    });
    let mut clears = Vec::new();
    while let Ok(command) = sent.try_recv() {
        if let Command::ClearTerminalAttention { machine, clear } = command {
            clears.push((machine, clear));
        }
    }
    if ATTENTION_NOTIFICATIONS_PROTOCOL.is_some() {
        assert_eq!(
            clears,
            vec![(
                "lab".to_owned(),
                ClearTerminalAttention {
                    terminal: id,
                    revision: TerminalAttentionRevision::new(7),
                }
            )],
            "once, at the revision shown"
        );
    } else {
        assert!(clears.is_empty(), "no machine admits claims yet");
    }
}
