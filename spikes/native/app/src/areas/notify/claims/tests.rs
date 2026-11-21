//! A machine's answer decides which notices are told: what it admitted and
//! what its first inbox page did not hold, never the rest. While ADR
//! 0089's protocol is unassigned no machine is asked, so a notice is told
//! at once.

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
