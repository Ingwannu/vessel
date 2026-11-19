//! Settings › Notifications in English and Korean: the four ways in the
//! page's order and words, the chosen one checked, each row's change, and
//! the rows' words. A language change keeps the choice and a live input.
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::localization::{Locale, Translator};
use vessel_app_view::prefs::{Alerts, PreferenceChange};

use super::super::choices::{List, change, label, rows};
use crate::shell::{Shell, language_tests};

fn listed(shell: &Shell) -> Vec<(String, bool)> {
    rows(List::Alerts, shell, &Tokens::of(Scheme::Dark))
        .iter()
        .map(|row| (row.label.to_string(), row.checked))
        .collect()
}

#[gpui::test]
fn the_ways_to_tell_reword_while_the_choice_stays(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, _cx| {
            shell.prefs.settings.notifications.alerts = Alerts::Sound;
            assert_eq!(label(List::Alerts, shell), "Sound only");
            assert_eq!(
                listed(shell),
                [
                    ("Off".to_owned(), false),
                    ("Notifications only".to_owned(), false),
                    ("Sound only".to_owned(), true),
                    ("Notifications with sound".to_owned(), false),
                ]
            );
            for (index, alerts) in Alerts::ALL.into_iter().enumerate() {
                assert_eq!(
                    change(List::Alerts, index),
                    Some(PreferenceChange::Alerts(alerts))
                );
            }
            assert_eq!(change(List::Alerts, Alerts::ALL.len()), None);
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            assert_eq!(shell.prefs.settings.notifications.alerts, Alerts::Sound);
            assert_eq!(label(List::Alerts, shell), "소리만");
            assert_eq!(
                listed(shell),
                [
                    ("끄기".to_owned(), false),
                    ("알림만".to_owned(), false),
                    ("소리만".to_owned(), true),
                    ("알림과 소리".to_owned(), false),
                ]
            );
        },
    );
}

#[test]
fn the_rows_have_their_words_in_both_languages() {
    let en = Translator::get(Locale::from_tag("en").unwrap())
        .unwrap()
        .notifications();
    let ko = Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .notifications();
    assert_eq!(en.alerts(), "Session notifications");
    assert_eq!(ko.alerts(), "세션 알림");
    assert_eq!(en.title(), "Count in the window's title");
    assert_eq!(ko.title(), "창 제목에 개수 표시");
    assert_eq!(en.test_send(), "Send");
    assert_eq!(ko.test_send(), "보내기");
    assert_eq!(en.test_title(), "Vessel notification test");
    assert_eq!(en.test_body(), "If you can see this, notifications work.");
    assert_eq!(en.test_sent(), "Test notification sent");
    assert_eq!(ko.test_sent(), "시험 알림을 보냈습니다");
    assert_eq!(en.untitled(), "Untitled session");
    assert_eq!(en.test_hint(), "Send a notification using these settings");
    assert_eq!(
        en.test_handed(),
        "Vessel handed the notification to the operating system."
    );
    assert_eq!(en.test_failed(), "Unable to send test notification");
    assert_eq!(ko.test_failed(), "시험 알림을 보내지 못했습니다");
}
