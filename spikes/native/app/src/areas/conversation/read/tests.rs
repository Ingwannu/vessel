//! The read of the session shown, on a machine that admits ADR 0089's
//! protocol, goes as one `inbox.read` and no `session.mark`; below it the
//! read mark stays (app review of claims row 4, note 1). A second check
//! inside the same second sends nothing more.

use gpui::TestAppContext;
use vessel_app_link::{Act, Command, Link};
use vessel_app_view::{Fleet, Machine, Reach};
use vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL;
use vessel_proto::{
    EventRecord, EventSeq, HarnessKind, Mark, SessionEvent, SessionId, SessionInfo, SessionMode,
    SessionSpec,
};

use crate::areas::conversation::area::Shown;
use crate::shell::{Shell, language_tests};

/// The session as the node describes it, unread.
fn unread(session: SessionId) -> SessionInfo {
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let mut info = SessionInfo::new(session, spec, chrono::DateTime::UNIX_EPOCH);
    info.marks.unread = true;
    info
}

#[gpui::test]
fn a_read_on_a_claiming_machine_is_one_inbox_read_and_no_mark(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut sent) = Link::detached(16).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.run_until_parked();
    cx.update(|window, _| window.activate_window());
    while sent.try_recv().is_ok() {}
    let session = SessionId::random();
    shell.update_in(cx, |shell, window, cx| {
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
        let area = &mut shell.areas.conversation;
        area.shown = Some(Shown {
            machine: "lab".to_owned(),
            session,
        });
        area.info = Some(Box::new(unread(session)));
        let _change = area.timeline.apply(&[EventRecord {
            seq: EventSeq::new(0),
            at: "2026-11-21T09:00:00Z".parse().unwrap(),
            event: SessionEvent::Forked {
                from: SessionId::random(),
                after: None,
            },
        }]);
        shell.conversation_read(window, cx);
        shell.conversation_read(window, cx);
    });
    let (mut inbox, mut marks) = (Vec::new(), 0_usize);
    while let Ok(command) = sent.try_recv() {
        if matches!(
            command,
            Command::Act {
                act: Act::Mark(Mark::Read { .. }),
                ..
            }
        ) {
            marks = marks.saturating_add(1);
        }
        if let Command::ReadInbox(read) = command {
            inbox.push((read.machine, read.session, read.through));
        }
    }
    if ATTENTION_NOTIFICATIONS_PROTOCOL.is_some() {
        assert_eq!(inbox, vec![("lab".to_owned(), session, EventSeq::new(1))]);
        assert_eq!(marks, 0, "no session.mark beside the inbox read");
    } else {
        assert!(inbox.is_empty(), "no machine admits claims yet");
        assert_eq!(marks, 1, "the read mark stays");
    }
}
