//! A terminal notice is claimed only by a window that would tell it, and
//! shown only when the machine admits it: titled in Paseo's words, with the
//! terminal's title (never its command) as the body. A click on a folder
//! terminal that no session names as a path asks the node.

use std::path::PathBuf;

use gpui::TestAppContext;
use vessel_app_link::Command;
use vessel_app_link::Link;
use vessel_app_link::claims::TerminalAnswer;
use vessel_app_link::terminal_holder::Located;
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

/// Only a window that shows the system's notifications claims (ADR 0089,
/// "Turned on", item 5): one with alerts off, one that only sounds, and one
/// whose system cannot show a notification never take a notice from a
/// window that can.
#[gpui::test]
fn a_window_that_does_not_notify_never_claims(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut sent) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.run_until_parked();
    // What the shell asks as it opens is not this test's.
    while sent.try_recv().is_ok() {}
    for (alerts, reaches) in [
        (Alerts::Off, true),
        (Alerts::Sound, true),
        (Alerts::System, false),
    ] {
        shell.update(cx, |shell, cx| {
            shell.prefs.settings.notifications.alerts = alerts;
            shell.areas.notify.reaches_system(reaches);
            shell.terminal_notices_found(
                "lab".to_owned(),
                vec![notice(NoticeKind::Finished, "t")],
                cx,
            );
        });
        assert!(
            sent.try_recv().is_err(),
            "{alerts:?}, system reached {reaches}: nothing is claimed"
        );
    }
    shell.update(cx, |shell, cx| {
        shell.prefs.settings.notifications.alerts = Alerts::System;
        shell.areas.notify.reaches_system(true);
        shell.terminal_notices_found(
            "lab".to_owned(),
            vec![notice(NoticeKind::Finished, "t")],
            cx,
        );
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

/// The lab, online at this protocol, listing `sessions`.
fn lab(sessions: Vec<vessel_app_view::Card>) -> Machine {
    Machine {
        connection_epoch: None,
        node: None,
        name: "lab".to_owned(),
        reach: Reach::Online,
        sessions,
        agents: Vec::new(),
        protocol: Some(vessel_proto::PROTOCOL_VERSION),
        version: None,
    }
}

/// A session of the lab's whose workspace is the folder `cwd`.
fn in_folder(session: SessionId, cwd: &str) -> vessel_app_view::Card {
    vessel_app_view::Card {
        session,
        structured: true,
        live: true,
        title: "notes".to_owned(),
        machine: "lab".to_owned(),
        agent: "Claude".to_owned(),
        lane: None,
        phrase: vessel_app_view::board::Phrase::Starting,
        attention: vessel_app_view::board::Attention::AtRest,
        at: chrono::Utc::now(),
        detail: vessel_app_view::board::CardDetail {
            cwd: Some(cwd.to_owned()),
            ..vessel_app_view::board::CardDetail::default()
        },
    }
}

/// Whether the session shown is `session`.
fn shows(shell: &Shell, session: SessionId) -> bool {
    matches!(
        crate::areas::layout::current_key(shell),
        Ok(vessel_app_view::layout::LayoutKey::Session { session: at, .. }) if at == session
    )
}

/// A folder terminal that no session's folder names as a path (the node
/// keeps it canonical; the session keeps it as given, here through a link)
/// is asked of the node. The session it names opens on the next frame; a
/// find for another click is dropped.
#[gpui::test]
fn a_folder_terminal_no_session_names_as_a_path_is_located_by_the_node(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut sent) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.run_until_parked();
    while sent.try_recv().is_ok() {}
    let (session, terminal) = (SessionId::random(), TerminalId::random());
    let place = TerminalPlace::Folder {
        path: PathBuf::from("/real/notes"),
    };
    shell.update_in(cx, |shell, window, cx| {
        shell.fleet = Fleet {
            machines: vec![lab(vec![in_folder(session, "/home/me/notes")])],
            read_at: None,
        };
        shell.terminal_notice_clicked("lab", terminal, &place, window, cx);
        assert!(
            !shows(shell, session),
            "nothing opens before the node answers"
        );
    });
    let mut asked = Vec::new();
    while let Ok(command) = sent.try_recv() {
        if let Command::LocateTerminal(locate) = command {
            asked.push((locate.machine, locate.terminal, locate.candidates));
        }
    }
    assert_eq!(
        asked,
        vec![(
            "lab".to_owned(),
            terminal,
            vec![(session, PathBuf::from("/home/me/notes"))]
        )]
    );
    shell.update_in(cx, |shell, window, cx| {
        let found = |terminal| Located {
            machine: "lab".to_owned(),
            terminal,
            session,
        };
        shell.terminal_located(found(TerminalId::random()));
        shell.settle_terminal_notice(window, cx);
        assert!(!shows(shell, session), "another click's find is dropped");
        shell.terminal_located(found(terminal));
        shell.settle_terminal_notice(window, cx);
        assert!(shows(shell, session));
    });
}
